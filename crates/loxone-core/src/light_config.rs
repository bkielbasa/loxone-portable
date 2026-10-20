use crate::factory::generate_loxone_uuid;
use crate::model::{LoxoneProject, PageBlock, RawNode, TreeNode};
use crate::writer::ProjectEdit;

pub const DIMMER_Q_BASE: i64 = 1_610_612_736; // 0x6000_0000

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightCircuitType {
    /// `"0"` or absent — Przełącznik / Switch
    Switch,
    /// `"24"` — Dimmer 0-100%
    Dimmer0To100,
    /// `"25"` — Dimmer 0-10V
    Dimmer0To10V,
    /// `"26"` — Dimmer 1-10V
    Dimmer1To10V,
    /// `"31"` — RGB
    Rgb,
    /// `"32"` — Lumitech DMX
    LumitechDmx,
    /// `"16384"` — Smart aktor / Smart Actuator
    SmartActuator,
}

impl LightCircuitType {
    pub fn from_xml_code(code: Option<&str>) -> Self {
        match code.map(str::trim) {
            None | Some("") | Some("0") => Self::Switch,
            Some("24") => Self::Dimmer0To100,
            Some("25") => Self::Dimmer0To10V,
            Some("26") => Self::Dimmer1To10V,
            Some("31") => Self::Rgb,
            Some("32") => Self::LumitechDmx,
            Some("16384") => Self::SmartActuator,
            _ => Self::Dimmer0To100,
        }
    }

    pub fn to_xml_code(self) -> Option<&'static str> {
        match self {
            Self::Switch => None,
            Self::Dimmer0To100 => Some("24"),
            Self::Dimmer0To10V => Some("25"),
            Self::Dimmer1To10V => Some("26"),
            Self::Rgb => Some("31"),
            Self::LumitechDmx => Some("32"),
            Self::SmartActuator => Some("16384"),
        }
    }

    pub fn is_dimmer_like(self) -> bool {
        !matches!(self, Self::Switch)
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Switch,
            Self::Dimmer0To100,
            Self::Dimmer0To10V,
            Self::Dimmer1To10V,
            Self::Rgb,
            Self::LumitechDmx,
            Self::SmartActuator,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightCircuit {
    pub index: usize,
    pub name: String,
    pub circuit_type: LightCircuitType,
    pub pres_m: Option<String>,
}

impl LightCircuit {
    pub fn code_label(&self) -> String {
        format!("Lc{}", self.index + 1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightScene {
    pub uuid: String,
    pub name: String,
    pub short_name: String,
    pub sid: i32,
    pub cid: i32,
    pub read_only: bool,
    pub mix_mode: bool,
    pub add_mode: bool,
    pub move_default: bool,
    pub circuit_pcts: [u8; 8],
    pub raw_q: [i64; 18],
}

impl LightScene {
    pub fn new_user_scene(name: &str, sid: i32, circuits: &[LightCircuit; 8]) -> Self {
        let mut raw_q = [0i64; 18];
        for i in 0..8 {
            if circuits[i].circuit_type.is_dimmer_like() || i < 6 {
                raw_q[i] = DIMMER_Q_BASE;
            }
        }
        Self {
            uuid: generate_loxone_uuid(),
            name: name.to_string(),
            short_name: String::new(),
            sid,
            cid: 11 + sid.clamp(1, 88),
            read_only: false,
            mix_mode: true,
            add_mode: false,
            move_default: false,
            circuit_pcts: [0; 8],
            raw_q,
        }
    }

    pub fn input_label(&self) -> String {
        if (1..=8).contains(&self.sid) {
            format!("Dis{}", self.sid)
        } else {
            "-".to_string()
        }
    }

    fn from_raw_node(node: &RawNode) -> Self {
        let sid = node
            .attr("SID")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);
        let cid = node
            .attr("CID")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(12);
        let read_only = node.attr("ReadOnly") == Some("true") || sid == 778;
        let mut raw_q = [0i64; 18];
        let mut circuit_pcts = [0u8; 8];
        for i in 0..18 {
            let key = format!("Q{}", i + 1);
            let q_val = node
                .attr(&key)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0);
            raw_q[i] = q_val;
            if i < 8 {
                circuit_pcts[i] = decode_q_to_pct(q_val);
            }
        }
        Self {
            uuid: node
                .attr("UUID")
                .map(str::to_string)
                .unwrap_or_else(generate_loxone_uuid),
            name: node.attr("Name").unwrap_or_default().to_string(),
            short_name: node.attr("SName").unwrap_or_default().to_string(),
            sid,
            cid,
            read_only,
            mix_mode: node.attr("MixMod") == Some("true"),
            add_mode: node.attr("AddMod") == Some("true"),
            move_default: node.attr("Move") == Some("true"),
            circuit_pcts,
            raw_q,
        }
    }

    fn to_raw_node(&self, outputs_count: usize, circuits: &[LightCircuit; 8]) -> RawNode {
        let mut attributes = Vec::new();
        attributes.push(("Name".to_string(), self.name.clone()));
        if !self.short_name.is_empty() {
            attributes.push(("SName".to_string(), self.short_name.clone()));
        }
        attributes.push(("UUID".to_string(), self.uuid.clone()));
        if self.read_only {
            attributes.push(("ReadOnly".to_string(), "true".to_string()));
        }
        if self.move_default {
            attributes.push(("Move".to_string(), "true".to_string()));
        }
        attributes.push(("Outputs".to_string(), outputs_count.to_string()));
        if self.mix_mode && !self.read_only {
            attributes.push(("MixMod".to_string(), "true".to_string()));
        }
        if self.add_mode && !self.read_only {
            attributes.push(("AddMod".to_string(), "true".to_string()));
        }
        attributes.push(("SID".to_string(), self.sid.to_string()));
        attributes.push(("CID".to_string(), self.cid.to_string()));

        for i in 0..outputs_count.min(18) {
            let key = format!("Q{}", i + 1);
            let encoded = if i < 8 {
                encode_pct_to_q(
                    self.circuit_pcts[i],
                    circuits[i].circuit_type,
                    self.raw_q[i],
                )
            } else {
                self.raw_q[i]
            };
            attributes.push((key, encoded.to_string()));
        }

        RawNode {
            tag: "LightsceneC".to_string(),
            attributes,
            text: None,
            children: Vec::new(),
        }
    }
}

pub fn decode_q_to_pct(q_val: i64) -> u8 {
    if q_val >= DIMMER_Q_BASE {
        (q_val - DIMMER_Q_BASE).clamp(0, 100) as u8
    } else if q_val == 1 {
        100
    } else if (2..=100).contains(&q_val) {
        q_val as u8
    } else {
        0
    }
}

pub fn encode_pct_to_q(pct: u8, circuit_type: LightCircuitType, original_raw_q: i64) -> i64 {
    let clamped = pct.min(100);
    if circuit_type.is_dimmer_like() || original_raw_q >= DIMMER_Q_BASE {
        DIMMER_Q_BASE + i64::from(clamped)
    } else if clamped > 0 {
        1
    } else {
        0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomationAction {
    /// `0` — Kontrola stałego natężenia światła / Constant brightness control
    ConstantBrightness,
    /// `1` — Dopasuj jasność / Adjust brightness
    AdjustBrightness,
    /// `2` — Nie zmieniaj jasności / Do not change brightness
    KeepBrightness,
}

impl AutomationAction {
    pub fn from_xml_code(code: i32) -> Self {
        match code {
            0 => Self::ConstantBrightness,
            1 => Self::AdjustBrightness,
            _ => Self::KeepBrightness,
        }
    }

    pub fn to_xml_code(self) -> i32 {
        match self {
            Self::ConstantBrightness => 0,
            Self::AdjustBrightness => 1,
            Self::KeepBrightness => 2,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::KeepBrightness,
            Self::AdjustBrightness,
            Self::ConstantBrightness,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightAutomationRule {
    /// Operating mode ID (`778` = `<Brak trybu pracy>`, `-1000` = `Tryb nocny`, etc.)
    pub operating_mode_id: i32,
    /// Scene `SID` (`1..=99`, `777`, `778`)
    pub scene_sid: i32,
    pub action: AutomationAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HclMode {
    /// `M="0"` — Dostosuj krzywą do wschodu i zachodu słońca
    SunriseSunset,
    /// `M="1"` — Stała krzywa w zdefiniowanym okresie
    FixedPeriod,
}

impl HclMode {
    pub fn from_xml_code(code: Option<&str>) -> Self {
        match code {
            Some("1") => Self::FixedPeriod,
            _ => Self::SunriseSunset,
        }
    }

    pub fn to_xml_code(self) -> &'static str {
        match self {
            Self::SunriseSunset => "0",
            Self::FixedPeriod => "1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HclConfig {
    pub hcl_type: i32,
    pub mode: HclMode,
    pub tmin_kelvin: u32,
    pub tmax_kelvin: u32,
    pub from_seconds: u32,
    pub to_seconds: u32,
}

impl Default for HclConfig {
    fn default() -> Self {
        Self {
            hcl_type: 4,
            mode: HclMode::SunriseSunset,
            tmin_kelvin: 2700,
            tmax_kelvin: 6500,
            from_seconds: 19871, // 05:31
            to_seconds: 72290,   // 20:04
        }
    }
}

impl HclConfig {
    pub fn from_hm(&self) -> (u32, u32) {
        let h = (self.from_seconds / 3600).min(23);
        let m = ((self.from_seconds % 3600) / 60).min(59);
        (h, m)
    }

    pub fn set_from_hm(&mut self, h: u32, m: u32) {
        self.from_seconds = h.min(23) * 3600 + m.min(59) * 60;
    }

    pub fn to_hm(&self) -> (u32, u32) {
        let h = (self.to_seconds / 3600).min(23);
        let m = ((self.to_seconds % 3600) / 60).min(59);
        (h, m)
    }

    pub fn set_to_hm(&mut self, h: u32, m: u32) {
        self.to_seconds = h.min(23) * 3600 + m.min(59) * 60;
    }

    pub fn sample_kelvin_at_hour(&self, hour: f32) -> f32 {
        let t_sec = hour.clamp(0.0, 24.0) * 3600.0;
        let start = self.from_seconds as f32;
        let end = (self.to_seconds as f32).max(start + 3600.0);
        let min_k = self.tmin_kelvin as f32;
        let max_k = self.tmax_kelvin.max(self.tmin_kelvin) as f32;
        if t_sec <= start || t_sec >= end {
            return min_k;
        }
        let u = (t_sec - start) / (end - start);
        // Smooth raised-cosine / bell curve peaking around midday
        let bell = (std::f32::consts::PI * u).sin().powf(1.2);
        min_k + (max_k - min_k) * bell
    }

    fn from_raw_node(node: &RawNode) -> Self {
        Self {
            hcl_type: node
                .attr("T")
                .and_then(|v| v.parse().ok())
                .unwrap_or(4),
            mode: HclMode::from_xml_code(node.attr("M")),
            tmin_kelvin: node
                .attr("TMIN")
                .and_then(|v| v.parse().ok())
                .unwrap_or(2700),
            tmax_kelvin: node
                .attr("TMAX")
                .and_then(|v| v.parse().ok())
                .unwrap_or(6500),
            from_seconds: node
                .attr("FROM")
                .and_then(|v| v.parse().ok())
                .unwrap_or(19871),
            to_seconds: node
                .attr("TO")
                .and_then(|v| v.parse().ok())
                .unwrap_or(72290),
        }
    }

    fn to_raw_node(&self) -> RawNode {
        RawNode {
            tag: "HCL".to_string(),
            attributes: vec![
                ("T".to_string(), self.hcl_type.to_string()),
                ("M".to_string(), self.mode.to_xml_code().to_string()),
                ("TMIN".to_string(), self.tmin_kelvin.to_string()),
                ("TMAX".to_string(), self.tmax_kelvin.to_string()),
                ("FROM".to_string(), self.from_seconds.to_string()),
                ("TO".to_string(), self.to_seconds.to_string()),
            ],
            text: None,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightControllerConfig {
    pub block_uuid: String,
    pub block_title: String,
    pub circuits: [LightCircuit; 8],
    pub favorite_colors_fc: String,
    pub outputs_count: usize,
    pub scenes: Vec<LightScene>,
    pub automation_rules: Vec<LightAutomationRule>,
    pub hcl: HclConfig,
}

impl LightControllerConfig {
    pub fn is_light_controller_block(block_type: &str) -> bool {
        matches!(
            block_type,
            "LightController2" | "LightControllerV2" | "LightController"
        )
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let circuits = std::array::from_fn(|i| {
            let name_key = format!("NameAI{i}");
            let cap_key = format!("CapAI{i}");
            let pres_key = format!("PresM{i}");
            let name = block
                .attr(&name_key)
                .unwrap_or("nieznane")
                .to_string();
            let circuit_type = LightCircuitType::from_xml_code(block.attr(&cap_key));
            let pres_m = block.attr(&pres_key).map(str::to_string);
            LightCircuit {
                index: i,
                name,
                circuit_type,
                pres_m,
            }
        });

        let mut favorite_colors_fc = "0040FFFF,008080FF,00FFFF80,0040FF40".to_string();
        let mut outputs_count = 18usize;
        let mut scenes = Vec::new();

        if let Some(scenes_container) = block
            .extra_children
            .iter()
            .find(|c| c.tag == "LightscenesC")
        {
            if let Some(fc) = scenes_container.attr("FC") {
                favorite_colors_fc = fc.to_string();
            }
            if let Some(out_n) = scenes_container
                .attr("Outputs")
                .and_then(|v| v.parse::<usize>().ok())
            {
                outputs_count = out_n;
            }
            for child in &scenes_container.children {
                if child.tag == "LightsceneC" {
                    scenes.push(LightScene::from_raw_node(child));
                }
            }
        }

        if !scenes.iter().any(|s| s.sid == 778) {
            let mut off_scene = LightScene::new_user_scene("Wył.", 778, &circuits);
            off_scene.read_only = true;
            off_scene.mix_mode = false;
            off_scene.cid = 11;
            scenes.push(off_scene);
        }

        let mut automation_rules = Vec::new();
        if let Some(ls_cfg) = block.extra_children.iter().find(|c| c.tag == "LSConfig") {
            let op_ms: Vec<i32> = ls_cfg
                .attr("OpM")
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            let sc_mvs: Vec<i32> = ls_cfg
                .attr("ScMv")
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            let sc_acs: Vec<i32> = ls_cfg
                .attr("ScAc")
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            let len = op_ms.len().max(sc_mvs.len()).max(sc_acs.len());
            for i in 0..len {
                automation_rules.push(LightAutomationRule {
                    operating_mode_id: op_ms.get(i).copied().unwrap_or(778),
                    scene_sid: sc_mvs.get(i).copied().unwrap_or(778),
                    action: AutomationAction::from_xml_code(sc_acs.get(i).copied().unwrap_or(2)),
                });
            }
        }

        let hcl = block
            .extra_children
            .iter()
            .find(|c| c.tag == "HCL")
            .map(HclConfig::from_raw_node)
            .unwrap_or_default();

        Self {
            block_uuid: block.uuid.clone(),
            block_title: block.title.clone(),
            circuits,
            favorite_colors_fc,
            outputs_count,
            scenes,
            automation_rules,
            hcl,
        }
    }

    pub fn next_available_sid(&self) -> i32 {
        for cand in 1..=99 {
            if !self.scenes.iter().any(|s| s.sid == cand) {
                return cand;
            }
        }
        99
    }

    pub fn to_project_edits(&self, original_block: &PageBlock) -> ProjectEdit {
        let mut edits = Vec::new();

        for i in 0..8 {
            let c = &self.circuits[i];
            let name_key = format!("NameAI{i}");
            let cap_key = format!("CapAI{i}");
            let pres_key = format!("PresM{i}");

            let is_configured =
                (c.name != "nieznane" && !c.name.trim().is_empty()) || c.circuit_type.is_dimmer_like();
            let desired_name = if c.name != "nieznane" && !c.name.trim().is_empty() {
                Some(c.name.clone())
            } else {
                None
            };
            let desired_cap = c.circuit_type.to_xml_code().map(str::to_string);
            let desired_pres = if is_configured {
                Some(c.pres_m.clone().unwrap_or_else(|| "65535".to_string()))
            } else {
                None
            };

            if original_block.attr(&name_key) != desired_name.as_deref() {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: self.block_uuid.clone(),
                    attr_name: name_key,
                    value: desired_name,
                });
            }
            if original_block.attr(&cap_key) != desired_cap.as_deref() {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: self.block_uuid.clone(),
                    attr_name: cap_key,
                    value: desired_cap,
                });
            }
            if original_block.attr(&pres_key) != desired_pres.as_deref() {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: self.block_uuid.clone(),
                    attr_name: pres_key,
                    value: desired_pres,
                });
            }
        }

        let scenes_node = RawNode {
            tag: "LightscenesC".to_string(),
            attributes: vec![
                ("FC".to_string(), self.favorite_colors_fc.clone()),
                ("Outputs".to_string(), self.outputs_count.to_string()),
                ("Num".to_string(), self.scenes.len().to_string()),
            ],
            text: None,
            children: self
                .scenes
                .iter()
                .map(|s| s.to_raw_node(self.outputs_count, &self.circuits))
                .collect(),
        };

        let op_m_str = self
            .automation_rules
            .iter()
            .map(|r| r.operating_mode_id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sc_mv_str = self
            .automation_rules
            .iter()
            .map(|r| r.scene_sid.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sc_ac_str = self
            .automation_rules
            .iter()
            .map(|r| r.action.to_xml_code().to_string())
            .collect::<Vec<_>>()
            .join(",");

        let ls_config_node = RawNode {
            tag: "LSConfig".to_string(),
            attributes: vec![
                ("OpM".to_string(), op_m_str),
                ("ScMv".to_string(), sc_mv_str),
                ("ScAc".to_string(), sc_ac_str),
            ],
            text: None,
            children: Vec::new(),
        };

        let hcl_node = self.hcl.to_raw_node();

        let mut new_extra = Vec::new();
        let mut inserted_scenes = false;
        let mut inserted_ls_cfg = false;
        let mut inserted_hcl = false;

        for existing in &original_block.extra_children {
            match existing.tag.as_str() {
                "LightscenesC" => {
                    if !inserted_scenes {
                        new_extra.push(scenes_node.clone());
                        inserted_scenes = true;
                    }
                }
                "LSConfig" => {
                    if !inserted_ls_cfg {
                        new_extra.push(ls_config_node.clone());
                        inserted_ls_cfg = true;
                    }
                }
                "HCL" => {
                    if !inserted_hcl {
                        new_extra.push(hcl_node.clone());
                        inserted_hcl = true;
                    }
                }
                _ => new_extra.push(existing.clone()),
            }
        }

        if !inserted_scenes {
            new_extra.push(scenes_node);
        }
        if !inserted_ls_cfg {
            new_extra.push(ls_config_node);
        }
        if !inserted_hcl {
            new_extra.push(hcl_node);
        }

        edits.push(ProjectEdit::SetExtraChildren {
            control_uuid: self.block_uuid.clone(),
            extra_children: new_extra,
        });

        ProjectEdit::Batch(edits)
    }
}

pub fn collect_operating_modes(project: &LoxoneProject, no_mode_label: &str) -> Vec<(i32, String)> {
    let mut modes = vec![(778, no_mode_label.to_string())];
    for root in &project.system_tree {
        collect_modes_recursive(root, &mut modes);
    }
    modes
}

fn collect_modes_recursive(node: &TreeNode, out: &mut Vec<(i32, String)>) {
    if node.control_type == "Mode" {
        let mode_id = node
            .attr("Mode")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        if !out.iter().any(|(id, _)| *id == mode_id) {
            out.push((mode_id, node.title.clone()));
        }
    }
    for child in &node.children {
        collect_modes_recursive(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_loxone_xml;
    use crate::writer::{apply_edit_with_inverse, serialize_loxone_xml};

    const SALON_LC_UUID: &str = "19eb19a5-029f-27e5-ffff51535bcccc04";

    #[test]
    fn test_parse_light_controller_config_from_klimczaki() {
        let xml = std::fs::read_to_string("../../Klimczaki.Loxone")
            .expect("Failed to read Klimczaki.Loxone");
        let project = parse_loxone_xml(&xml).expect("Failed to parse Klimczaki.Loxone");
        let lc_block = project
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == SALON_LC_UUID)
            .expect("Salon + jadalnia LightController2 must exist");

        let cfg = LightControllerConfig::from_block(lc_block);

        // Tab 1: Obwody świetlne (light02.png / light03.png)
        assert_eq!(cfg.circuits[0].name, "jadalnia: nad stołem");
        assert_eq!(cfg.circuits[0].circuit_type, LightCircuitType::Dimmer0To100);
        assert_eq!(cfg.circuits[1].name, "salon: oświetlenie nad sofą");
        assert_eq!(cfg.circuits[1].circuit_type, LightCircuitType::Dimmer0To100);
        assert_eq!(cfg.circuits[2].name, "salon: spoty do okoła");
        assert_eq!(cfg.circuits[2].circuit_type, LightCircuitType::Dimmer0To100);
        assert_eq!(cfg.circuits[3].name, "salon: zasłony");
        assert_eq!(cfg.circuits[3].circuit_type, LightCircuitType::Dimmer0To100);
        for i in 4..8 {
            assert_eq!(cfg.circuits[i].name, "nieznane");
            assert_eq!(cfg.circuits[i].circuit_type, LightCircuitType::Switch);
        }

        // Tab 2: Nastroje (light01.png)
        assert_eq!(cfg.scenes.len(), 5);
        assert_eq!(cfg.scenes[0].name, "salon: scopy");
        assert_eq!(cfg.scenes[0].short_name, "SCOPY");
        assert_eq!(cfg.scenes[0].sid, 1);
        assert_eq!(cfg.scenes[0].input_label(), "Dis1");
        assert!(cfg.scenes[0].mix_mode);
        assert_eq!(cfg.scenes[0].circuit_pcts, [0, 0, 2, 0, 0, 0, 0, 0]);

        assert_eq!(cfg.scenes[1].name, "Ledy");
        assert_eq!(cfg.scenes[1].short_name, "NAD STOLEM W JADALNI");
        assert_eq!(cfg.scenes[1].sid, 2);
        assert_eq!(cfg.scenes[1].circuit_pcts, [0, 0, 0, 36, 0, 0, 0, 0]);

        assert_eq!(cfg.scenes[2].name, "Oświetlenie główne");
        assert_eq!(cfg.scenes[2].short_name, "GLOWNE");
        assert_eq!(cfg.scenes[2].sid, 3);
        assert_eq!(cfg.scenes[2].circuit_pcts, [0, 100, 0, 0, 0, 0, 0, 0]);

        assert_eq!(cfg.scenes[3].name, "Nad stołem");
        assert_eq!(cfg.scenes[3].short_name, "NAD STOLEM W JADALNI");
        assert_eq!(cfg.scenes[3].sid, 4);
        assert_eq!(cfg.scenes[3].circuit_pcts, [100, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(cfg.scenes[4].name, "Wył.");
        assert_eq!(cfg.scenes[4].sid, 778);
        assert!(cfg.scenes[4].read_only);
        assert_eq!(cfg.scenes[4].input_label(), "-");

        // Tab 3: Automatyka (light04.png)
        assert_eq!(cfg.automation_rules.len(), 2);
        assert_eq!(cfg.automation_rules[0].operating_mode_id, -1000);
        assert_eq!(cfg.automation_rules[0].scene_sid, 2);
        assert_eq!(
            cfg.automation_rules[0].action,
            AutomationAction::KeepBrightness
        );
        assert_eq!(cfg.automation_rules[1].operating_mode_id, 778);
        assert_eq!(cfg.automation_rules[1].scene_sid, 2);

        let modes = collect_operating_modes(&project, "<Brak trybu pracy>");
        assert!(modes
            .iter()
            .any(|(id, title)| *id == -1000 && title == "Tryb nocny"));

        // Tab 4: Ustawienia kontroli światła dziennego (light05.png)
        assert_eq!(cfg.hcl.mode, HclMode::SunriseSunset);
        assert_eq!(cfg.hcl.from_hm(), (5, 31));
        assert_eq!(cfg.hcl.to_hm(), (20, 4));
        assert_eq!(cfg.hcl.tmin_kelvin, 2700);
        assert_eq!(cfg.hcl.tmax_kelvin, 6500);
    }

    #[test]
    fn test_light_controller_edit_and_roundtrip() {
        let xml = std::fs::read_to_string("../../Klimczaki.Loxone")
            .expect("Failed to read Klimczaki.Loxone");
        let mut project = parse_loxone_xml(&xml).expect("Failed to parse Klimczaki.Loxone");
        let lc_block = project
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == SALON_LC_UUID)
            .expect("Salon + jadalnia LightController2 must exist")
            .clone();

        let mut cfg = LightControllerConfig::from_block(&lc_block);
        cfg.circuits[4].name = "salon: kinkiet".to_string();
        cfg.circuits[4].circuit_type = LightCircuitType::Dimmer0To100;
        cfg.scenes[0].circuit_pcts[2] = 15;
        cfg.automation_rules[0].scene_sid = 778;
        cfg.hcl.tmin_kelvin = 3000;

        let edit = cfg.to_project_edits(&lc_block);
        let inverse = apply_edit_with_inverse(&mut project, &edit).expect("apply edit");

        let serialized = serialize_loxone_xml(&project).expect("serialize");
        let reparsed = parse_loxone_xml(&serialized).expect("reparse");
        let updated_block = reparsed
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == SALON_LC_UUID)
            .unwrap();
        let updated_cfg = LightControllerConfig::from_block(updated_block);

        assert_eq!(updated_cfg.circuits[4].name, "salon: kinkiet");
        assert_eq!(
            updated_cfg.circuits[4].circuit_type,
            LightCircuitType::Dimmer0To100
        );
        assert_eq!(updated_cfg.scenes[0].circuit_pcts[2], 15);
        assert_eq!(updated_cfg.automation_rules[0].scene_sid, 778);
        assert_eq!(updated_cfg.hcl.tmin_kelvin, 3000);

        // Undo restores original state
        apply_edit_with_inverse(&mut project, &inverse).expect("undo");
        let restored_block = project
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == SALON_LC_UUID)
            .unwrap();
        let restored_cfg = LightControllerConfig::from_block(restored_block);
        assert_eq!(restored_cfg.circuits[4].name, "nieznane");
        assert_eq!(restored_cfg.scenes[0].circuit_pcts[2], 2);
        assert_eq!(restored_cfg.automation_rules[0].scene_sid, 2);
        assert_eq!(restored_cfg.hcl.tmin_kelvin, 2700);
    }
}
