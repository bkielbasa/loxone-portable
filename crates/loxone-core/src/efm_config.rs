use crate::factory::generate_loxone_uuid;
use crate::model::{LoxoneProject, PageBlock, RawNode};
use crate::writer::ProjectEdit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EfmNodeType {
    /// `t="1"` — Sieć / Grid
    Grid,
    /// `t="2"` — Bateria / Storage
    Battery,
    /// `t="3"` — Konsument / Consumer
    Consumer,
    /// `t="4"` — Produkcja energii / Energy Production
    Production,
    /// `t="5"` — Grupa / Group
    Group,
}

impl EfmNodeType {
    pub fn from_xml_code(code: &str) -> Self {
        match code.trim() {
            "1" => Self::Grid,
            "2" => Self::Battery,
            "3" => Self::Consumer,
            "4" => Self::Production,
            "5" => Self::Group,
            _ => Self::Consumer,
        }
    }

    pub fn to_xml_code(self) -> &'static str {
        match self {
            Self::Grid => "1",
            Self::Battery => "2",
            Self::Consumer => "3",
            Self::Production => "4",
            Self::Group => "5",
        }
    }

    pub fn all_child_types() -> &'static [Self] {
        &[
            Self::Grid,
            Self::Production,
            Self::Battery,
            Self::Consumer,
            Self::Group,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatInterval {
    /// `m="0"` — Wył. / Off
    Off,
    /// `m="8"` — Interwał 5 min
    Min5,
    /// `m="9"` — Interwał 15 min
    Min15,
    /// `m="10"` — Interwał 30 min
    Min30,
    /// `m="11"` — Interwał 60 min
    Min60,
}

impl StatInterval {
    pub fn from_xml_code(code: &str) -> Self {
        match code.trim() {
            "0" => Self::Off,
            "8" => Self::Min5,
            "9" => Self::Min15,
            "10" => Self::Min30,
            "11" => Self::Min60,
            _ => Self::Min60,
        }
    }

    pub fn to_xml_code(self) -> &'static str {
        match self {
            Self::Off => "0",
            Self::Min5 => "8",
            Self::Min15 => "9",
            Self::Min30 => "10",
            Self::Min60 => "11",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Off,
            Self::Min5,
            Self::Min15,
            Self::Min30,
            Self::Min60,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EfmNode {
    pub uuid: String,
    pub node_type: EfmNodeType,
    pub title: String,
    pub ctrl_uuid: String,
    pub calc_rest: bool,
    pub rest_name: String,
    pub live_state: Option<String>,
    pub efm_state: Option<String>,
    pub children: Vec<EfmNode>,
}

impl EfmNode {
    pub fn new_child(title: &str, node_type: EfmNodeType, ctrl_uuid: &str) -> Self {
        let state_uuid = generate_loxone_uuid();
        Self {
            uuid: generate_loxone_uuid(),
            node_type,
            title: title.to_string(),
            ctrl_uuid: ctrl_uuid.to_string(),
            calc_rest: false,
            rest_name: String::new(),
            live_state: Some(state_uuid.clone()),
            efm_state: Some(state_uuid),
            children: Vec::new(),
        }
    }

    fn from_raw_node(node: &RawNode, fallback_title: &str) -> Self {
        let node_type = node
            .attr("t")
            .map(EfmNodeType::from_xml_code)
            .unwrap_or(EfmNodeType::Group);
        let title = node.attr("tit").unwrap_or(fallback_title).to_string();
        let children = node
            .children
            .iter()
            .filter(|c| c.tag == "nd")
            .map(|c| Self::from_raw_node(c, ""))
            .collect();
        Self {
            uuid: node
                .attr("u")
                .map(str::to_string)
                .unwrap_or_else(generate_loxone_uuid),
            node_type,
            title,
            ctrl_uuid: node.attr("ctrlUuid").unwrap_or_default().to_string(),
            calc_rest: node.attr("rst") == Some("true"),
            rest_name: node.attr("rstName").unwrap_or_default().to_string(),
            live_state: node.attr("liveState").map(str::to_string),
            efm_state: node.attr("efmState").map(str::to_string),
            children,
        }
    }

    fn to_raw_node(&self, is_root: bool, root_ctrl_uuid: &str) -> RawNode {
        let mut attributes = Vec::new();
        attributes.push(("u".to_string(), self.uuid.clone()));
        attributes.push(("t".to_string(), self.node_type.to_xml_code().to_string()));
        if !is_root {
            attributes.push(("tit".to_string(), self.title.clone()));
            if let Some(ls) = &self.live_state {
                attributes.push(("liveState".to_string(), ls.clone()));
            }
            attributes.push(("ctrlUuid".to_string(), self.ctrl_uuid.clone()));
            if let Some(es) = &self.efm_state {
                attributes.push(("efmState".to_string(), es.clone()));
            }
        } else {
            let ctrl = if self.ctrl_uuid.is_empty() {
                root_ctrl_uuid.to_string()
            } else {
                self.ctrl_uuid.clone()
            };
            attributes.push(("ctrlUuid".to_string(), ctrl));
        }

        if is_root || self.node_type == EfmNodeType::Group {
            if self.calc_rest {
                attributes.push(("rst".to_string(), "true".to_string()));
            }
            if is_root || self.calc_rest || !self.rest_name.is_empty() {
                attributes.push(("rstName".to_string(), self.rest_name.clone()));
            }
        }

        let children = self
            .children
            .iter()
            .map(|c| c.to_raw_node(false, root_ctrl_uuid))
            .collect();

        RawNode {
            tag: if is_root {
                "root".to_string()
            } else {
                "nd".to_string()
            },
            attributes,
            text: None,
            children,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EfmObjectCandidate {
    pub uuid: String,
    pub title: String,
    pub room_title: String,
    pub display_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EfmConfig {
    pub block_uuid: String,
    pub root: EfmNode,
    pub unit_power: String,
    pub unit_energy: String,
    /// `id="3"` — Statystyki dla mocy
    pub stat_power_interval: StatInterval,
    /// `id="1"` — Statystyki dotyczące energii
    pub stat_energy_interval: StatInterval,
    /// `id="2"` — Statystyki dotyczące samowystarczalności i zużycia własnego
    pub stat_self_sufficiency_interval: StatInterval,
}

impl EfmConfig {
    pub fn is_efm_block(block_type: &str) -> bool {
        block_type == "EFM"
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let root = block
            .extra_children
            .iter()
            .find(|c| c.tag == "root")
            .map(|r| EfmNode::from_raw_node(r, &block.title))
            .unwrap_or_else(|| EfmNode {
                uuid: generate_loxone_uuid(),
                node_type: EfmNodeType::Group,
                title: block.title.clone(),
                ctrl_uuid: block.uuid.clone(),
                calc_rest: true,
                rest_name: String::new(),
                live_state: None,
                efm_state: None,
                children: Vec::new(),
            });

        let unit_power = block
            .attr("Un1")
            .unwrap_or("<v.3>kW")
            .to_string();
        let unit_energy = block
            .attr("Un2")
            .unwrap_or("<v.1>kWh")
            .to_string();

        let mut stat_power_interval = StatInterval::Min30;
        let mut stat_energy_interval = StatInterval::Min60;
        let mut stat_self_sufficiency_interval = StatInterval::Min60;

        for child in &block.extra_children {
            if child.tag == "statGrp" {
                let interval = child
                    .attr("m")
                    .map(StatInterval::from_xml_code)
                    .unwrap_or(StatInterval::Min60);
                match child.attr("id") {
                    Some("3") => stat_power_interval = interval,
                    Some("1") => stat_energy_interval = interval,
                    Some("2") => stat_self_sufficiency_interval = interval,
                    _ => {}
                }
            }
        }

        Self {
            block_uuid: block.uuid.clone(),
            root,
            unit_power,
            unit_energy,
            stat_power_interval,
            stat_energy_interval,
            stat_self_sufficiency_interval,
        }
    }

    pub fn to_project_edits(&self, original_block: &PageBlock) -> ProjectEdit {
        let mut edits = Vec::new();

        if self.root.title != original_block.title {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: self.block_uuid.clone(),
                attr_name: "Title".to_string(),
                value: Some(self.root.title.clone()),
            });
        }
        if original_block.attr("Un1") != Some(self.unit_power.as_str()) {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: self.block_uuid.clone(),
                attr_name: "Un1".to_string(),
                value: Some(self.unit_power.clone()),
            });
        }
        if original_block.attr("Un2") != Some(self.unit_energy.as_str()) {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: self.block_uuid.clone(),
                attr_name: "Un2".to_string(),
                value: Some(self.unit_energy.clone()),
            });
        }

        let mut new_extra: Vec<RawNode> = Vec::new();
        let mut inserted_root = false;
        let mut inserted_stats = false;

        let root_node = self.root.to_raw_node(true, &self.block_uuid);
        let stat3 = make_stat_grp("3", self.stat_power_interval, false);
        let stat1 = make_stat_grp("1", self.stat_energy_interval, true);
        let stat2 = make_stat_grp("2", self.stat_self_sufficiency_interval, true);

        for existing in &original_block.extra_children {
            if existing.tag == "root" {
                if !inserted_root {
                    new_extra.push(root_node.clone());
                    inserted_root = true;
                }
            } else if existing.tag == "statGrp" {
                if !inserted_stats {
                    new_extra.push(stat3.clone());
                    new_extra.push(stat1.clone());
                    new_extra.push(stat2.clone());
                    inserted_stats = true;
                }
            } else {
                new_extra.push(existing.clone());
            }
        }

        if !inserted_root {
            new_extra.push(root_node);
        }
        if !inserted_stats {
            new_extra.push(stat3);
            new_extra.push(stat1);
            new_extra.push(stat2);
        }

        edits.push(ProjectEdit::SetExtraChildren {
            control_uuid: self.block_uuid.clone(),
            extra_children: new_extra,
        });

        ProjectEdit::Batch(edits)
    }
}

fn make_stat_grp(id: &str, interval: StatInterval, acc: bool) -> RawNode {
    let mut attributes = vec![
        ("id".to_string(), id.to_string()),
        ("m".to_string(), interval.to_xml_code().to_string()),
    ];
    if acc && interval != StatInterval::Off {
        attributes.push(("acc".to_string(), "true".to_string()));
    }
    RawNode {
        tag: "statGrp".to_string(),
        attributes,
        text: None,
        children: Vec::new(),
    }
}

pub fn resolve_efm_object_label(project: &LoxoneProject, ctrl_uuid: &str) -> String {
    if ctrl_uuid.is_empty() {
        return String::new();
    }
    for page in &project.pages {
        for block in &page.blocks {
            if block.uuid == ctrl_uuid {
                let room_title = block
                    .room_uuid
                    .as_deref()
                    .or(page.room_uuid.as_deref())
                    .and_then(|ru| project.places.get(ru))
                    .map(|p| format_efm_room_title(&p.title))
                    .unwrap_or_default();
                if room_title.is_empty() {
                    return block.title.clone();
                } else {
                    return format!("{} ({})", block.title, room_title);
                }
            }
        }
    }
    ctrl_uuid.to_string()
}

fn format_efm_room_title(raw_room: &str) -> &str {
    // In Loxone Config (feat02-energy.png), Place "Rozdzielnica" with PType="5" (Technical)
    // or room title is shown; note in feat02-energy.png "Rozdzielnica" was renamed or mapped to "Techniczne"
    // or if title is "Rozdzielnica", let's check if Loxone displays the room title directly.
    raw_room
}

pub fn collect_efm_candidate_objects(project: &LoxoneProject) -> Vec<EfmObjectCandidate> {
    let mut out = Vec::new();
    for page in &project.pages {
        for block in &page.blocks {
            if block.uuid.is_empty() || block.block_type == "EFM" {
                continue;
            }
            let is_energy_block = block.block_type.starts_with("Meter")
                || matches!(
                    block.block_type.as_str(),
                    "Wallbox" | "Wallbox2" | "LoadManager" | "SolarController" | "SpotPriceOptimizer"
                )
                || block.attr("Un1").is_some()
                || block.attr("Un2").is_some();
            if !is_energy_block {
                continue;
            }
            let room_title = block
                .room_uuid
                .as_deref()
                .or(page.room_uuid.as_deref())
                .and_then(|ru| project.places.get(ru))
                .map(|p| p.title.clone())
                .unwrap_or_default();
            let display_label = if room_title.is_empty() {
                block.title.clone()
            } else {
                format!("{} ({})", block.title, room_title)
            };
            out.push(EfmObjectCandidate {
                uuid: block.uuid.clone(),
                title: block.title.clone(),
                room_title,
                display_label,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_loxone_xml;
    use crate::writer::{apply_edit_with_inverse, serialize_loxone_xml};

    const EFM_UUID: &str = "1d04656b-0128-8d7a-ffff51535bcccc04";

    #[test]
    fn test_parse_efm_config_from_klimczaki() {
        let xml = std::fs::read_to_string("../../Klimczaki.Loxone")
            .expect("Failed to read Klimczaki.Loxone");
        let project = parse_loxone_xml(&xml).expect("Failed to parse Klimczaki.Loxone");
        let efm_block = project
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == EFM_UUID)
            .expect("EFM block must exist");

        let cfg = EfmConfig::from_block(efm_block);
        assert_eq!(cfg.root.title, "Monitor przepływu energii");
        assert_eq!(cfg.root.node_type, EfmNodeType::Group);
        assert!(cfg.root.calc_rest);
        assert_eq!(cfg.root.rest_name, "");
        assert_eq!(cfg.root.children.len(), 2);

        assert_eq!(cfg.root.children[0].title, "Fotowoltaika");
        assert_eq!(cfg.root.children[0].node_type, EfmNodeType::Production);
        assert_eq!(
            cfg.root.children[0].ctrl_uuid,
            "1d04650d-0257-512c-ffff87bec3265e7f"
        );

        assert_eq!(cfg.root.children[1].title, "Licznik główny");
        assert_eq!(cfg.root.children[1].node_type, EfmNodeType::Grid);
        assert_eq!(
            cfg.root.children[1].ctrl_uuid,
            "1d0040f7-0341-6cb9-ffff51535bcccc04"
        );

        assert_eq!(cfg.unit_power, "<v.3>kW");
        assert_eq!(cfg.unit_energy, "<v.1>kWh");
        assert_eq!(cfg.stat_power_interval, StatInterval::Min30);
        assert_eq!(cfg.stat_energy_interval, StatInterval::Min60);
        assert_eq!(cfg.stat_self_sufficiency_interval, StatInterval::Min60);

        let candidates = collect_efm_candidate_objects(&project);
        assert!(candidates.iter().any(|c| c.title == "Fotowoltaika"));
        assert!(candidates.iter().any(|c| c.title == "Licznik główny"));
    }

    #[test]
    fn test_efm_config_edit_and_roundtrip() {
        let xml = std::fs::read_to_string("../../Klimczaki.Loxone")
            .expect("Failed to read Klimczaki.Loxone");
        let mut project = parse_loxone_xml(&xml).expect("Failed to parse Klimczaki.Loxone");
        let efm_block = project
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == EFM_UUID)
            .expect("EFM block must exist")
            .clone();

        let mut cfg = EfmConfig::from_block(&efm_block);
        cfg.root.rest_name = "Pozostałe odbiorniki".to_string();
        cfg.stat_power_interval = StatInterval::Min15;
        cfg.root.children.push(EfmNode::new_child(
            "Wallbox Garaż",
            EfmNodeType::Consumer,
            "1d0040f7-0341-6cb9-ffff51535bcccc04",
        ));

        let edit = cfg.to_project_edits(&efm_block);
        let inverse = apply_edit_with_inverse(&mut project, &edit).expect("apply edit");

        let serialized = serialize_loxone_xml(&project).expect("serialize");
        let reparsed = parse_loxone_xml(&serialized).expect("reparse");
        let updated_block = reparsed
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == EFM_UUID)
            .expect("EFM block after reparse");

        // Embedded MeterAbsBi child must still be preserved!
        assert!(updated_block
            .extra_children
            .iter()
            .any(|c| c.tag == "C" && c.attr("Type") == Some("MeterAbsBi")));

        let updated_cfg = EfmConfig::from_block(updated_block);
        assert_eq!(updated_cfg.root.rest_name, "Pozostałe odbiorniki");
        assert_eq!(updated_cfg.stat_power_interval, StatInterval::Min15);
        assert_eq!(updated_cfg.root.children.len(), 3);
        assert_eq!(updated_cfg.root.children[2].title, "Wallbox Garaż");
        assert_eq!(updated_cfg.root.children[2].node_type, EfmNodeType::Consumer);

        // Undo restores original state
        apply_edit_with_inverse(&mut project, &inverse).expect("undo");
        let restored_block = project
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == EFM_UUID)
            .unwrap();
        let restored_cfg = EfmConfig::from_block(restored_block);
        assert_eq!(restored_cfg.root.children.len(), 2);
        assert_eq!(restored_cfg.root.rest_name, "");
        assert_eq!(restored_cfg.stat_power_interval, StatInterval::Min30);
    }
}
