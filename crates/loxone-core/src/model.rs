use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub struct RawNode {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
    pub text: Option<String>,
    pub children: Vec<RawNode>,
}

impl RawNode {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn set_attr(&mut self, name: &str, value: Option<&str>) {
        match value {
            Some(val) => {
                if let Some((_, existing_val)) =
                    self.attributes.iter_mut().find(|(k, _)| k == name)
                {
                    *existing_val = val.to_string();
                } else {
                    self.attributes.push((name.to_string(), val.to_string()));
                }
            }
            None => {
                self.attributes.retain(|(k, _)| k != name);
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectMetadata {
    pub title: String,
    pub uuid: String,
    pub config_version: String,
    pub control_list_version: String,
    pub creator: String,
    pub street: String,
    pub town: String,
    pub postcode: String,
    pub country: String,
    pub latitude: String,
    pub longitude: String,
    pub currency: String,
    pub app_id: String,
    pub miniserver_ip: Option<String>,
    pub miniserver_serial: Option<String>,
    pub miniserver_hostname: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub uuid: String,
    pub title: String,
    pub cat_group: Option<String>,
    pub bk_color: Option<[u8; 3]>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub uuid: String,
    pub title: String,
    pub sqm: Option<String>,
    pub p_type: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKind {
    Input,
    Parameter,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireLink {
    pub source_pin_uuid: String,
    pub flags: Option<String>,
}

impl WireLink {
    /// Returns true if this link is marked `FLG="1"` (hidden internal object link or reverse API link).
    pub fn is_hidden(&self) -> bool {
        self.flags.as_deref() == Some("1")
    }

    /// Returns true if this link is marked `FLG="2"` (detached / wireless reference in Loxone Config).
    pub fn is_detached_ref(&self) -> bool {
        self.flags.as_deref() == Some("2")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectorPin {
    pub uuid: String,
    pub key: String,
    pub default_value: Option<String>,
    pub pin_kind: PinKind,
    pub incoming_links: Vec<WireLink>,
}

impl ConnectorPin {
    /// Returns true if this pin has at least one non-hidden (`FLG != "1"`) incoming wire link.
    pub fn has_visible_incoming_links(&self) -> bool {
        self.incoming_links.iter().any(|l| !l.is_hidden())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageBlock {
    pub uuid: String,
    pub block_type: String,
    pub title: String,
    pub has_position: bool,
    pub px: f32,
    pub py: f32,
    pub px2: f32,
    pub py2: f32,
    pub color_rgb: Option<[u8; 3]>,
    pub ref_uuid: Option<String>,
    pub extra_ref_uuids: Vec<String>,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
    pub text_content: Option<String>,
    pub pins: Vec<ConnectorPin>,
    pub raw_attributes: Vec<(String, String)>,
    pub extra_children: Vec<RawNode>,
}

impl PageBlock {
    pub fn is_input_ref_bar(&self) -> bool {
        self.block_type.starts_with("InputRef")
    }

    pub fn is_output_ref_bar(&self) -> bool {
        self.block_type.starts_with("OutputRef") || self.block_type == "Memory"
    }

    pub fn is_ref_bar(&self) -> bool {
        self.is_input_ref_bar() || self.is_output_ref_bar()
    }

    pub fn is_note(&self) -> bool {
        matches!(self.block_type.as_str(), "Note" | "Text")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub uuid: String,
    pub title: String,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
    pub blocks: Vec<PageBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeSectionKind {
    Periphery,
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    pub uuid: String,
    pub control_type: String,
    pub title: String,
    pub section: TreeSectionKind,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
    pub pins: Vec<ConnectorPin>,
    pub raw_attributes: Vec<(String, String)>,
    pub extra_children: Vec<RawNode>,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Default)]
pub struct LoxoneProject {
    pub metadata: ProjectMetadata,
    pub categories: IndexMap<String, Category>,
    pub places: IndexMap<String, Place>,
    pub pages: Vec<Page>,
    pub periphery_tree: Vec<TreeNode>,
    pub system_tree: Vec<TreeNode>,
    pub xref: crate::xref::XrefIndex,
    pub raw_xml: Option<String>,
}

pub fn parse_rgb_color(cl: &str) -> Option<[u8; 3]> {
    let mut parts = cl.split(',').map(|p| p.trim().parse::<u8>().ok());
    let r = parts.next()??;
    let g = parts.next()??;
    let b = parts.next()??;
    if parts.next().is_some() {
        return None;
    }
    Some([r, g, b])
}

pub fn classify_pin_kind(
    key: &str,
    def: Option<&str>,
    has_incoming_links: bool,
    is_wire_source: bool,
) -> PinKind {
    if matches!(key, "API" | "OutputAPI") || is_wire_source {
        return PinKind::Output;
    }
    if has_incoming_links {
        return PinKind::Input;
    }
    if def.is_some() {
        return PinKind::Parameter;
    }
    let is_named_output = key == "Q"
        || key.starts_with('Q')
        || key == "AQ"
        || key.starts_with("AQ")
        || key.starts_with("Out")
        || key.starts_with("TQ")
        || key.starts_with("TxQ")
        || key.starts_with("Level")
        || matches!(
            key,
            "Scene"
                | "RoomOff"
                | "HouseOff"
                | "TargetPos"
                | "Light"
                | "Pulse"
                | "Rest"
                | "API"
                | "OutputAPI"
                | "ActiveLevel"
                | "LastStart"
                | "LastStop"
                | "ConfirmCause"
                | "ConfirmTime"
                | "OPf"
                | "OMr"
                | "OMr1"
                | "OMr2"
                | "OGpwr"
                | "OPpwr"
                | "OCpwr"
                | "OSpwr"
        )
        || key.starts_with("OR");
    if is_named_output {
        PinKind::Output
    } else {
        PinKind::Input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rgb_color() {
        assert_eq!(parse_rgb_color("255,0,84"), Some([255, 0, 84]));
        assert_eq!(parse_rgb_color("164, 112, 255"), Some([164, 112, 255]));
        assert_eq!(parse_rgb_color("0,0,0"), Some([0, 0, 0]));
        assert_eq!(parse_rgb_color("invalid"), None);
        assert_eq!(parse_rgb_color("255,0"), None);
    }

    #[test]
    fn test_classify_pin_kind() {
        assert_eq!(classify_pin_kind("Delay", Some("600"), false, false), PinKind::Parameter);
        assert_eq!(classify_pin_kind("Delay", Some("600"), true, false), PinKind::Input);
        assert_eq!(classify_pin_kind("I1", None, false, false), PinKind::Input);
        assert_eq!(classify_pin_kind("AI", None, true, false), PinKind::Input);
        assert_eq!(classify_pin_kind("Q", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("AQ1", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("OutAlarm", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("OutputAPI", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("OutputAPI", None, true, false), PinKind::Output);
        assert_eq!(classify_pin_kind("Level1", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("CustomPin", None, false, true), PinKind::Output);
    }
}
