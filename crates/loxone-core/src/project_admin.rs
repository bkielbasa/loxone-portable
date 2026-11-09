//! Project administration helpers: users, user groups & permissions,
//! operating modes and calendar entries.
//!
//! Format notes (observed in real `.Loxone` saves, see `Sample.Loxone`):
//! - Users: `<C Type="User">` under `<C Type="UserCaption">`. Fields `FiN` (first
//!   name), `LaN` (last name), `Ema` (e-mail), `Phn` (phone). Password hashes live
//!   in `<HP>` children and are preserved untouched.
//! - User groups: `<C Type="UserGroup">` under `UserGroupCaption`. Members are a
//!   comma separated list of user UUIDs in the `User` attribute. Granted
//!   permissions are `<PH NUM="n"><PRM REF="<permission uuid>" ACT="3"/></PH>`.
//! - Permissions: `<C Type="Permission" PT="n">` (fixed system rights).
//! - Operating modes: `<C Type="Mode" Mode="n">` under `ModeCaption`; user-defined
//!   modes use negative numbers.
//! - Calendar entries: `<C Type="CalendarEntry">` under `CalendarCaption` with
//!   `CalType` = the operating mode number it activates and a date range
//!   (`Month`/`Day` .. `Month2`/`Day2`).

use crate::factory::{generate_loxone_uuid, new_tree_node};
use crate::model::{ConnectorPin, LoxoneProject, PinKind, RawNode, TreeNode, TreeSectionKind};
use crate::writer::ProjectEdit;

/// Every tree node of the given `Type` in both trees (depth first, file order).
pub fn find_nodes_by_type<'a>(project: &'a LoxoneProject, ty: &str) -> Vec<&'a TreeNode> {
    fn walk<'a>(nodes: &'a [TreeNode], ty: &str, out: &mut Vec<&'a TreeNode>) {
        for n in nodes {
            if n.control_type == ty {
                out.push(n);
            }
            walk(&n.children, ty, out);
        }
    }
    let mut out = Vec::new();
    walk(&project.system_tree, ty, &mut out);
    walk(&project.periphery_tree, ty, &mut out);
    out
}

pub fn find_first_by_type<'a>(project: &'a LoxoneProject, ty: &str) -> Option<&'a TreeNode> {
    find_nodes_by_type(project, ty).into_iter().next()
}

pub fn find_node_by_uuid<'a>(project: &'a LoxoneProject, uuid: &str) -> Option<&'a TreeNode> {
    fn walk<'a>(nodes: &'a [TreeNode], uuid: &str) -> Option<&'a TreeNode> {
        for n in nodes {
            if n.uuid == uuid {
                return Some(n);
            }
            if let Some(found) = walk(&n.children, uuid) {
                return Some(found);
            }
        }
        None
    }
    walk(&project.system_tree, uuid).or_else(|| walk(&project.periphery_tree, uuid))
}

// ---------------------------------------------------------------------------
// Users & groups
// ---------------------------------------------------------------------------

/// Editable user fields: (XML attribute, English label).
pub const USER_FIELDS: &[(&str, &str)] = &[
    ("Title", "User name"),
    ("FiN", "First name"),
    ("LaN", "Last name"),
    ("Ema", "E-mail"),
    ("Phn", "Phone"),
];

pub fn group_member_uuids(group: &TreeNode) -> Vec<String> {
    group
        .attr("User")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Edit that adds or removes `user_uuid` from the group's `User` member list.
pub fn set_group_membership_edit(group: &TreeNode, user_uuid: &str, member: bool) -> ProjectEdit {
    let mut members = group_member_uuids(group);
    members.retain(|u| u != user_uuid);
    if member {
        members.push(user_uuid.to_string());
    }
    ProjectEdit::SetControlAttr {
        control_uuid: group.uuid.clone(),
        attr_name: "User".to_string(),
        value: Some(members.join(",")),
    }
}

/// Permission UUIDs granted to a group or user through its `<PH>` child.
pub fn granted_permission_refs(node: &TreeNode) -> Vec<String> {
    node.extra_children
        .iter()
        .filter(|c| c.tag == "PH")
        .flat_map(|ph| ph.children.iter())
        .filter(|prm| prm.tag == "PRM")
        .filter_map(|prm| prm.attr("REF").map(str::to_string))
        .collect()
}

/// Edit that grants or revokes one permission on a group/user, rebuilding `<PH>`
/// while keeping every other child (and other PRM entries with their extra
/// attributes such as `TIM`) intact.
pub fn set_permission_edit(node: &TreeNode, permission_uuid: &str, granted: bool) -> ProjectEdit {
    let mut children = node.extra_children.clone();
    let ph_idx = match children.iter().position(|c| c.tag == "PH") {
        Some(i) => i,
        None => {
            children.push(RawNode {
                tag: "PH".to_string(),
                attributes: vec![("NUM".to_string(), "0".to_string())],
                text: None,
                children: Vec::new(),
            });
            children.len() - 1
        }
    };
    {
        let ph = &mut children[ph_idx];
        ph.children
            .retain(|prm| !(prm.tag == "PRM" && prm.attr("REF") == Some(permission_uuid)));
        if granted {
            ph.children.push(RawNode {
                tag: "PRM".to_string(),
                attributes: vec![
                    ("REF".to_string(), permission_uuid.to_string()),
                    ("ACT".to_string(), "3".to_string()),
                ],
                text: None,
                children: Vec::new(),
            });
        }
        let n = ph.children.iter().filter(|c| c.tag == "PRM").count();
        ph.set_attr("NUM", Some(&n.to_string()));
    }
    if children[ph_idx].children.is_empty() {
        children.remove(ph_idx);
    }
    ProjectEdit::SetExtraChildren {
        control_uuid: node.uuid.clone(),
        extra_children: children,
    }
}

fn with_attrs(mut node: TreeNode, extra: &[(&str, &str)]) -> TreeNode {
    for (k, v) in extra {
        if let Some(slot) = node.raw_attributes.iter_mut().find(|(key, _)| key == k) {
            slot.1 = v.to_string();
        } else {
            node.raw_attributes.push((k.to_string(), v.to_string()));
        }
    }
    node
}

fn pin(key: &str, kind: PinKind) -> ConnectorPin {
    ConnectorPin {
        uuid: generate_loxone_uuid(),
        key: key.to_string(),
        default_value: None,
        inverted: false,
        pin_kind: kind,
        incoming_links: Vec::new(),
    }
}

/// Creates a new user under `UserCaption`. Returns the edit and the new UUID.
pub fn new_user_edit(project: &LoxoneProject, name: &str) -> Option<(ProjectEdit, String)> {
    let parent = find_first_by_type(project, "UserCaption")?;
    let mut node = with_attrs(
        new_tree_node("User", name),
        &[
            ("WF", "16384"),
            ("FiN", ""),
            ("LaN", ""),
            ("Ema", ""),
            ("Phn", ""),
            ("AppGroup", "1"),
        ],
    );
    node.control_type = "User".to_string();
    node.title = name.to_string();
    node.pins.clear();
    let uuid = node.uuid.clone();
    Some((
        ProjectEdit::AddTreeNode {
            section: TreeSectionKind::System,
            parent_uuid: Some(parent.uuid.clone()),
            node,
        },
        uuid,
    ))
}

/// Creates a new (empty) user group under `UserGroupCaption`.
pub fn new_user_group_edit(project: &LoxoneProject, name: &str) -> Option<(ProjectEdit, String)> {
    let parent = find_first_by_type(project, "UserGroupCaption")?;
    let mut attrs: Vec<(&str, String)> = vec![
        ("WF", "16384".to_string()),
        ("User", String::new()),
        ("UserGroups", String::new()),
    ];
    // New groups use the project's "always" access schedule like existing ones.
    if let Some(tmr) = find_nodes_by_type(project, "UserGroup")
        .iter()
        .find_map(|g| g.attr("TMR"))
    {
        attrs.push(("TMR", tmr.to_string()));
    }
    let attrs_ref: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let mut node = with_attrs(new_tree_node("UserGroup", name), &attrs_ref);
    node.control_type = "UserGroup".to_string();
    node.title = name.to_string();
    node.pins.clear();
    let uuid = node.uuid.clone();
    Some((
        ProjectEdit::AddTreeNode {
            section: TreeSectionKind::System,
            parent_uuid: Some(parent.uuid.clone()),
            node,
        },
        uuid,
    ))
}

/// Removing a user also strips it from every group's member list.
pub fn delete_user_edits(project: &LoxoneProject, user_uuid: &str) -> ProjectEdit {
    let mut edits = Vec::new();
    for g in find_nodes_by_type(project, "UserGroup") {
        if group_member_uuids(g).iter().any(|u| u == user_uuid) {
            edits.push(set_group_membership_edit(g, user_uuid, false));
        }
    }
    edits.push(ProjectEdit::RemoveTreeNode {
        node_uuid: user_uuid.to_string(),
    });
    ProjectEdit::Batch(edits)
}

// ---------------------------------------------------------------------------
// Operating modes & calendar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ModeInfo {
    pub uuid: String,
    pub title: String,
    pub number: i32,
    pub user_defined: bool,
}

pub fn operating_modes(project: &LoxoneProject) -> Vec<ModeInfo> {
    find_nodes_by_type(project, "Mode")
        .into_iter()
        .map(|m| {
            let number = m
                .attr("Mode")
                .and_then(|s| s.trim().parse::<i32>().ok())
                .unwrap_or(0);
            ModeInfo {
                uuid: m.uuid.clone(),
                title: m.title.clone(),
                number,
                user_defined: number < 0,
            }
        })
        .collect()
}

pub fn mode_title_for_number(project: &LoxoneProject, number: i32) -> Option<String> {
    operating_modes(project)
        .into_iter()
        .find(|m| m.number == number)
        .map(|m| m.title)
}

/// Next free negative mode number for a user-defined mode (-1, -2, …).
pub fn next_user_mode_number(project: &LoxoneProject) -> i32 {
    let used: std::collections::HashSet<i32> =
        operating_modes(project).iter().map(|m| m.number).collect();
    let mut n = -1;
    while used.contains(&n) {
        n -= 1;
    }
    n
}

pub fn new_operating_mode_edit(
    project: &LoxoneProject,
    name: &str,
) -> Option<(ProjectEdit, String)> {
    let parent = find_first_by_type(project, "ModeCaption")?;
    let number = next_user_mode_number(project).to_string();
    let mut node = with_attrs(
        new_tree_node("Mode", name),
        &[("Nio", "2"), ("WF", "16384"), ("Mode", number.as_str())],
    );
    node.control_type = "Mode".to_string();
    node.title = name.to_string();
    node.pins = vec![pin("I", PinKind::Input), pin("Q", PinKind::Output)];
    let uuid = node.uuid.clone();
    Some((
        ProjectEdit::AddTreeNode {
            section: TreeSectionKind::System,
            parent_uuid: Some(parent.uuid.clone()),
            node,
        },
        uuid,
    ))
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarEntryInfo {
    pub uuid: String,
    pub title: String,
    /// Operating mode number this entry activates (`CalType`).
    pub mode_number: i32,
    /// Recurrence code (`Mode` attribute); 4 = yearly date range (observed).
    pub recurrence: i32,
    pub month: u32,
    pub day: u32,
    pub month2: u32,
    pub day2: u32,
    pub year: Option<i32>,
    pub year2: Option<i32>,
}

fn attr_num<T: std::str::FromStr>(n: &TreeNode, key: &str) -> Option<T> {
    n.attr(key).and_then(|s| s.trim().parse::<T>().ok())
}

pub fn calendar_entries(project: &LoxoneProject) -> Vec<CalendarEntryInfo> {
    find_nodes_by_type(project, "CalendarEntry")
        .into_iter()
        .map(|n| CalendarEntryInfo {
            uuid: n.uuid.clone(),
            title: n.title.clone(),
            mode_number: attr_num(n, "CalType").unwrap_or(0),
            recurrence: attr_num(n, "Mode").unwrap_or(0),
            month: attr_num(n, "Month").unwrap_or(1),
            day: attr_num(n, "Day").unwrap_or(1),
            month2: attr_num(n, "Month2").unwrap_or(1),
            day2: attr_num(n, "Day2").unwrap_or(1),
            year: attr_num(n, "Year"),
            year2: attr_num(n, "Year2"),
        })
        .collect()
}

/// Human readable recurrence label. Only code 4 has been observed in real
/// projects (yearly recurring date range); the others are shown as codes.
pub fn recurrence_label(code: i32) -> String {
    match code {
        4 => "Yearly (date range)".to_string(),
        other => format!("Recurrence code {}", other),
    }
}

pub fn new_calendar_entry_edit(
    project: &LoxoneProject,
    title: &str,
    mode_number: i32,
) -> Option<(ProjectEdit, String)> {
    let parent = find_first_by_type(project, "CalendarCaption")?;
    let cal_type = mode_number.to_string();
    let mut node = with_attrs(
        new_tree_node("CalendarEntry", title),
        &[
            ("Nio", "2"),
            ("WF", "16384"),
            ("CalType", cal_type.as_str()),
            ("Mode", "4"),
            ("Month", "1"),
            ("Day", "1"),
            ("Month2", "1"),
            ("Day2", "1"),
            ("WeekDayInMonth", "1"),
        ],
    );
    node.control_type = "CalendarEntry".to_string();
    node.title = title.to_string();
    node.pins = vec![pin("I", PinKind::Input)];
    let uuid = node.uuid.clone();
    Some((
        ProjectEdit::AddTreeNode {
            section: TreeSectionKind::System,
            parent_uuid: Some(parent.uuid.clone()),
            node,
        },
        uuid,
    ))
}

/// Which operating modes are active on a given date according to the calendar
/// (yearly date ranges only, wrapping over new year) plus the weekday mode.
pub fn active_modes_on(project: &LoxoneProject, month: u32, day: u32, weekday_mon0: u32) -> Vec<String> {
    let mut out = Vec::new();
    // Weekday modes 3 (Monday) .. 9 (Sunday) are built in.
    if let Some(t) = mode_title_for_number(project, 3 + weekday_mon0 as i32) {
        out.push(t);
    }
    let key = month * 100 + day;
    for e in calendar_entries(project) {
        let start = e.month * 100 + e.day;
        let end = e.month2 * 100 + e.day2;
        let hit = if start <= end {
            key >= start && key <= end
        } else {
            key >= start || key <= end
        };
        if hit {
            let title = mode_title_for_number(project, e.mode_number).unwrap_or(e.title.clone());
            if !out.contains(&title) {
                out.push(title);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Project To-Do / Memo Manager & Loxone Autopilot Rule Designer (Stage 9)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectTodoItem {
    pub uuid: String,
    pub page_uuid: String,
    pub page_index: usize,
    pub page_title: String,
    pub title: String,
    pub text: String,
    pub px: f32,
    pub py: f32,
    pub is_resolved: bool,
}

pub fn collect_project_todos(project: &LoxoneProject) -> Vec<ProjectTodoItem> {
    let mut out = Vec::new();
    for (page_index, page) in project.pages.iter().enumerate() {
        for block in &page.blocks {
            if matches!(block.block_type.as_str(), "Memo" | "Note") {
                let done_attr = block
                    .raw_attributes
                    .iter()
                    .find(|(k, _)| k == "Done")
                    .map(|(_, v)| v == "true")
                    .unwrap_or(false);
                let text = block
                    .text_content
                    .clone()
                    .or_else(|| {
                        block
                            .raw_attributes
                            .iter()
                            .find(|(k, _)| k == "Text" || k == "Desc")
                            .map(|(_, v)| v.clone())
                    })
                    .unwrap_or_default();
                let is_resolved = done_attr || block.title.starts_with("[DONE]");
                out.push(ProjectTodoItem {
                    uuid: block.uuid.clone(),
                    page_uuid: page.uuid.clone(),
                    page_index,
                    page_title: page.title.clone(),
                    title: block.title.clone(),
                    text,
                    px: block.px,
                    py: block.py,
                    is_resolved,
                });
            }
        }
    }
    out
}

pub fn build_add_todo_edit(
    page_uuid: &str,
    title: &str,
    text: &str,
    px: f32,
    py: f32,
) -> (String, ProjectEdit) {
    let mut block = crate::factory::new_page_block("Memo", title, px, py);
    block.text_content = Some(text.to_string());
    block
        .raw_attributes
        .push(("Text".to_string(), text.to_string()));
    let uuid = block.uuid.clone();
    (
        uuid,
        ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.to_string(),
            block,
        },
    )
}

pub fn build_resolve_todo_edit(
    project: &LoxoneProject,
    todo_uuid: &str,
    delete_block: bool,
) -> Option<ProjectEdit> {
    let (page_uuid, block) = project.pages.iter().find_map(|p| {
        p.blocks
            .iter()
            .find(|b| b.uuid == todo_uuid)
            .map(|b| (p.uuid.clone(), b))
    })?;
    if delete_block {
        Some(ProjectEdit::RemovePageBlock {
            page_uuid,
            block_uuid: todo_uuid.to_string(),
        })
    } else {
        let new_title = if block.title.starts_with("[DONE] ") {
            block.title.clone()
        } else {
            format!("[DONE] {}", block.title)
        };
        Some(ProjectEdit::Batch(vec![
            ProjectEdit::SetControlAttr {
                control_uuid: todo_uuid.to_string(),
                attr_name: "Done".to_string(),
                value: Some("true".to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: todo_uuid.to_string(),
                attr_name: "Title".to_string(),
                value: Some(new_title),
            },
        ]))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutopilotRuleSpec {
    pub title: String,
    pub page_uuid: String,
    pub logic_op: String,
    pub condition_source_uuids: Vec<String>,
    pub target_block_uuid: String,
    pub target_pin_key: String,
    pub px: f32,
    pub py: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutopilotRuleInfo {
    pub uuid: String,
    pub title: String,
    pub logic_op: String,
    pub page_uuid: String,
    pub page_title: String,
    pub condition_count: usize,
}

pub fn collect_autopilot_rules(project: &LoxoneProject) -> Vec<AutopilotRuleInfo> {
    let mut out = Vec::new();
    for page in &project.pages {
        for block in &page.blocks {
            let is_ap = block
                .raw_attributes
                .iter()
                .any(|(k, v)| k == "AutopilotRule" && v == "true")
                || block.title.starts_with("[Autopilot]");
            if is_ap {
                let condition_count = block
                    .pins
                    .iter()
                    .map(|p| p.incoming_links.len())
                    .sum();
                out.push(AutopilotRuleInfo {
                    uuid: block.uuid.clone(),
                    title: block.title.clone(),
                    logic_op: block.block_type.clone(),
                    page_uuid: page.uuid.clone(),
                    page_title: page.title.clone(),
                    condition_count,
                });
            }
        }
    }
    out
}

pub fn build_autopilot_rule_edits(
    project: &LoxoneProject,
    spec: &AutopilotRuleSpec,
) -> Option<(String, ProjectEdit)> {
    let page = project.pages.iter().find(|p| p.uuid == spec.page_uuid)?;
    let gate_type = if spec.logic_op.trim().eq_ignore_ascii_case("OR") {
        "Or"
    } else {
        "And"
    };
    let rule_title = if spec.title.trim().starts_with("[Autopilot]") {
        spec.title.trim().to_string()
    } else {
        format!("[Autopilot] {}", spec.title.trim())
    };
    let mut gate_block = crate::factory::new_page_block(gate_type, &rule_title, spec.px, spec.py);
    gate_block
        .raw_attributes
        .push(("AutopilotRule".to_string(), "true".to_string()));
    let gate_uuid = gate_block.uuid.clone();
    let gate_out_pin_uuid = gate_block
        .pins
        .iter()
        .find(|p| p.key == "Q" || p.key == "O")
        .map(|p| p.uuid.clone())
        .unwrap_or_else(generate_loxone_uuid);

    let mut edits = vec![ProjectEdit::AddPageBlock {
        page_uuid: spec.page_uuid.clone(),
        block: gate_block,
    }];

    for (idx, cond_uuid) in spec.condition_source_uuids.iter().enumerate() {
        let in_pin_key = format!("I{}", idx + 1);
        if let Some(src_block) = page.blocks.iter().find(|b| b.uuid == *cond_uuid) {
            if let Some(src_pin) = src_block
                .pins
                .iter()
                .find(|p| matches!(p.key.as_str(), "Q" | "O" | "AQ"))
                .or_else(|| src_block.pins.first())
            {
                edits.push(ProjectEdit::AddWireLink {
                    target_block_uuid: gate_uuid.clone(),
                    target_pin_key: in_pin_key,
                    source_pin_uuid: src_pin.uuid.clone(),
                });
            }
        } else if let Some(tree_node) = find_node_by_uuid(project, cond_uuid) {
            let ref_block = crate::factory::new_reference_block(
                "InputRef",
                tree_node,
                (spec.px - 640.0).max(128.0),
                spec.py + (idx as f32) * 192.0,
            );
            let ref_out_pin_uuid = ref_block
                .pins
                .first()
                .map(|p| p.uuid.clone())
                .unwrap_or_else(generate_loxone_uuid);
            edits.push(ProjectEdit::AddPageBlock {
                page_uuid: spec.page_uuid.clone(),
                block: ref_block,
            });
            edits.push(ProjectEdit::AddWireLink {
                target_block_uuid: gate_uuid.clone(),
                target_pin_key: in_pin_key,
                source_pin_uuid: ref_out_pin_uuid,
            });
        }
    }

    if !spec.target_block_uuid.trim().is_empty() && !spec.target_pin_key.trim().is_empty() {
        edits.push(ProjectEdit::AddWireLink {
            target_block_uuid: spec.target_block_uuid.clone(),
            target_pin_key: spec.target_pin_key.clone(),
            source_pin_uuid: gate_out_pin_uuid,
        });
    }

    Some((gate_uuid, ProjectEdit::Batch(edits)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_loxone_xml;
    use crate::writer::{apply_edit, serialize_loxone_xml};

    const XML: &str = "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<ControlList Version=\"274\">\
<C Type=\"Document\" U=\"doc-1\" Title=\"Home\" Street=\"Old\" ConfigVersion=\"1\">\
<C Type=\"LoxCaption\" U=\"cap-users\" Title=\"Users\" CaptionType=\"20\">\
<C Type=\"UserCaption\" U=\"uc\" Title=\"Users\"><C Type=\"User\" U=\"u1\" Title=\"alice\" FiN=\"\"/></C>\
<C Type=\"LoxCaption\" U=\"pc\" Title=\"Rights\"><C Type=\"Permission\" U=\"p1\" Title=\"Config\" PT=\"1\"/></C>\
<C Type=\"UserGroupCaption\" U=\"gc\" Title=\"Groups\"><C Type=\"UserGroup\" U=\"g1\" Title=\"Admins\" User=\"\" TMR=\"t1\"/></C>\
</C>\
<C Type=\"ModeCaption\" U=\"mc\" Title=\"Modes\"><C Type=\"Mode\" U=\"m1\" Title=\"Holiday\" Mode=\"1\"/></C>\
<C Type=\"CalendarCaption\" U=\"cc\" Title=\"Calendar\"/>\
<C Type=\"Program\" U=\"prog\"/>\
</C></ControlList>";

    #[test]
    fn users_groups_modes_roundtrip() {
        let mut p = parse_loxone_xml(XML).unwrap();
        let g = find_first_by_type(&p, "UserGroup").unwrap().clone();
        apply_edit(&mut p, &set_group_membership_edit(&g, "u1", true)).unwrap();
        let g = find_first_by_type(&p, "UserGroup").unwrap().clone();
        assert_eq!(group_member_uuids(&g), vec!["u1".to_string()]);
        apply_edit(&mut p, &set_permission_edit(&g, "p1", true)).unwrap();
        let g = find_first_by_type(&p, "UserGroup").unwrap().clone();
        assert_eq!(granted_permission_refs(&g), vec!["p1".to_string()]);

        let (e, _) = new_user_edit(&p, "bob").unwrap();
        apply_edit(&mut p, &e).unwrap();
        assert_eq!(find_nodes_by_type(&p, "User").len(), 2);

        let (e, _) = new_operating_mode_edit(&p, "Party").unwrap();
        apply_edit(&mut p, &e).unwrap();
        assert!(operating_modes(&p).iter().any(|m| m.number == -1));

        let (e, _) = new_calendar_entry_edit(&p, "Xmas", 1).unwrap();
        apply_edit(&mut p, &e).unwrap();
        assert_eq!(calendar_entries(&p).len(), 1);

        apply_edit(
            &mut p,
            &ProjectEdit::SetControlAttr {
                control_uuid: "doc-1".into(),
                attr_name: "Street".into(),
                value: Some("New Street 1".into()),
            },
        )
        .unwrap();

        let xml = serialize_loxone_xml(&p).unwrap();
        assert!(xml.contains("New Street 1"));
        assert!(xml.contains("Title=\"bob\""));
        assert!(xml.contains("Title=\"Party\""));
        assert!(xml.contains("Title=\"Xmas\""));
        assert!(xml.contains("REF=\"p1\""));
        assert!(xml.contains("User=\"u1\""));
    }
}
