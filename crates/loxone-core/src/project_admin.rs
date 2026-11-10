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

fn find_all<'a>(project: &'a LoxoneProject) -> Vec<&'a TreeNode> {
    fn walk<'a>(nodes: &'a [TreeNode], out: &mut Vec<&'a TreeNode>) {
        for n in nodes {
            out.push(n);
            walk(&n.children, out);
        }
    }
    let mut out = Vec::new();
    walk(&project.system_tree, &mut out);
    walk(&project.periphery_tree, &mut out);
    out
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
            let is_text_todo = block.block_type == "Text"
                && (block.attr("Td") == Some("true") || block.title.contains("To-Do"));
            if matches!(block.block_type.as_str(), "Memo" | "Note") || is_text_todo {
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
    let mut block = crate::factory::new_page_block("Text", title, px, py);
    block.color_rgb = Some([255, 200, 100]);
    block.text_content = Some(text.to_string());
    if let Some((_, cl)) = block.raw_attributes.iter_mut().find(|(k, _)| k == "Cl") {
        *cl = "255,200,100".to_string();
    }
    if let Some((_, t)) = block.raw_attributes.iter_mut().find(|(k, _)| k == "Text") {
        *t = text.to_string();
    } else {
        block
            .raw_attributes
            .push(("Text".to_string(), text.to_string()));
    }
    if !block.raw_attributes.iter().any(|(k, _)| k == "Td") {
        block
            .raw_attributes
            .push(("Td".to_string(), "true".to_string()));
    }
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

// ---------------------------------------------------------------------------
// Multiplier Project & Template Inheritance Engine (Stage 13)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiplierUnitEntry {
    pub uuid: String,
    pub title: String,
    pub miniserver_serial: String,
    pub address: String,
    pub description: String,
    /// Pairs of `(template_device_uuid, unit_serial)` overridden in this unit.
    pub device_serial_overrides: Vec<(String, String)>,
}

pub fn is_multiplier_project(project: &LoxoneProject) -> bool {
    project
        .metadata
        .doc_attr("Multiplier")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
        || !find_nodes_by_type(project, "MultiplierUnit").is_empty()
        || find_all(project)
            .iter()
            .any(|n| n.attr("Type") == Some("MultiplierUnit") || n.attr("MultiplierUnit") == Some("true"))
}

pub fn collect_multiplier_units(project: &LoxoneProject) -> Vec<MultiplierUnitEntry> {
    let mut out = Vec::new();
    for node in find_all(project) {
        let is_mult_unit = node.control_type == "MultiplierUnit"
            || node.attr("MultiplierUnit") == Some("true");
        if !is_mult_unit {
            continue;
        }
        let mut overrides = Vec::new();
        for ch in &node.extra_children {
            if ch.tag == "Override" {
                if let (Some(r), Some(sn)) = (ch.attr("Ref"), ch.attr("Serial")) {
                    overrides.push((r.to_string(), sn.to_string()));
                }
            }
        }
        out.push(MultiplierUnitEntry {
            uuid: node.uuid.clone(),
            title: node.title.clone(),
            miniserver_serial: node.attr("Serial").unwrap_or_default().to_string(),
            address: node
                .attr("Address")
                .or_else(|| node.attr("Host"))
                .unwrap_or_default()
                .to_string(),
            description: node.attr("Desc").unwrap_or_default().to_string(),
            device_serial_overrides: overrides,
        });
    }
    out
}

pub fn build_upsert_multiplier_unit_edit(
    project: &LoxoneProject,
    unit: &MultiplierUnitEntry,
) -> (String, ProjectEdit) {
    let doc_uuid = project
        .metadata
        .doc_attr("U")
        .unwrap_or("doc-1")
        .to_string();
    let unit_uuid = if unit.uuid.trim().is_empty() {
        generate_loxone_uuid()
    } else {
        unit.uuid.trim().to_string()
    };

    let extra_children: Vec<RawNode> = unit
        .device_serial_overrides
        .iter()
        .filter(|(r, s)| !r.trim().is_empty() && !s.trim().is_empty())
        .map(|(r, s)| RawNode {
            tag: "Override".to_string(),
            attributes: vec![
                ("Ref".to_string(), r.trim().to_string()),
                ("Serial".to_string(), s.trim().to_ascii_uppercase()),
            ],
            text: None,
            children: Vec::new(),
        })
        .collect();

    let mut edits = vec![ProjectEdit::SetControlAttr {
        control_uuid: doc_uuid,
        attr_name: "Multiplier".to_string(),
        value: Some("true".to_string()),
    }];

    if find_node_by_uuid(project, &unit_uuid).is_some() {
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: unit_uuid.clone(),
            attr_name: "Title".to_string(),
            value: Some(unit.title.trim().to_string()),
        });
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: unit_uuid.clone(),
            attr_name: "Serial".to_string(),
            value: Some(unit.miniserver_serial.trim().to_ascii_uppercase()),
        });
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: unit_uuid.clone(),
            attr_name: "Address".to_string(),
            value: Some(unit.address.trim().to_string()),
        });
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: unit_uuid.clone(),
            attr_name: "Desc".to_string(),
            value: (!unit.description.trim().is_empty())
                .then(|| unit.description.trim().to_string()),
        });
        edits.push(ProjectEdit::SetExtraChildren {
            control_uuid: unit_uuid.clone(),
            extra_children,
        });
    } else {
        let net_caption_uuid = find_all(project)
            .into_iter()
            .find(|n| n.control_type == "LoxCaption" && n.attr("CaptionType") == Some("6"))
            .map(|n| n.uuid.clone());
        let node_type = if net_caption_uuid.is_some() {
            "NetworkDevice"
        } else {
            "MultiplierUnit"
        };
        let mut node = new_tree_node(node_type, unit.title.trim());
        node.uuid = unit_uuid.clone();
        node.raw_attributes = vec![
            ("Type".to_string(), node_type.to_string()),
            ("V".to_string(), "178".to_string()),
            ("U".to_string(), unit_uuid.clone()),
            ("Title".to_string(), unit.title.trim().to_string()),
            ("WF".to_string(), "16384".to_string()),
            ("MultiplierUnit".to_string(), "true".to_string()),
            (
                "Serial".to_string(),
                unit.miniserver_serial.trim().to_ascii_uppercase(),
            ),
            ("Address".to_string(), unit.address.trim().to_string()),
        ];
        if !unit.description.trim().is_empty() {
            node.raw_attributes
                .push(("Desc".to_string(), unit.description.trim().to_string()));
        }
        node.extra_children = extra_children;
        edits.push(ProjectEdit::AddTreeNode {
            section: TreeSectionKind::System,
            parent_uuid: net_caption_uuid,
            node,
        });
    }

    (unit_uuid, ProjectEdit::Batch(edits))
}

pub fn build_remove_multiplier_unit_edit(unit_uuid: &str) -> ProjectEdit {
    ProjectEdit::RemoveTreeNode {
        node_uuid: unit_uuid.to_string(),
    }
}

/// Instantiates a standalone `.Loxone` project for a specific Multiplier unit by cloning
/// the master template and applying the unit's Miniserver serial/address and device serial overrides.
pub fn instantiate_multiplier_unit_project(
    project: &LoxoneProject,
    unit_uuid: &str,
) -> Option<LoxoneProject> {
    let unit = collect_multiplier_units(project)
        .into_iter()
        .find(|u| u.uuid == unit_uuid)?;

    let mut derived = project.clone();
    let doc_uuid = derived
        .metadata
        .doc_attr("U")
        .unwrap_or("doc-1")
        .to_string();
    let new_title = if derived.metadata.title.trim().is_empty() {
        unit.title.clone()
    } else {
        format!("{} — {}", derived.metadata.title.trim(), unit.title)
    };

    let mut edits = vec![
        ProjectEdit::SetControlAttr {
            control_uuid: doc_uuid.clone(),
            attr_name: "Title".to_string(),
            value: Some(new_title),
        },
        ProjectEdit::SetControlAttr {
            control_uuid: doc_uuid,
            attr_name: "Multiplier".to_string(),
            value: None,
        },
    ];

    if let Some(ms_node) = find_first_by_type(&derived, "LoxLIVE") {
        let ms_uuid = ms_node.uuid.clone();
        if !unit.miniserver_serial.trim().is_empty() {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: ms_uuid.clone(),
                attr_name: "Serial".to_string(),
                value: Some(unit.miniserver_serial.trim().to_ascii_uppercase()),
            });
        }
        if !unit.address.trim().is_empty() {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: ms_uuid,
                attr_name: "IntAddr".to_string(),
                value: Some(unit.address.trim().to_string()),
            });
        }
    }

    for (dev_uuid, unit_serial) in &unit.device_serial_overrides {
        if find_node_by_uuid(&derived, dev_uuid).is_some() {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: dev_uuid.clone(),
                attr_name: "Serial".to_string(),
                value: Some(unit_serial.trim().to_ascii_uppercase()),
            });
        }
    }

    let _ = crate::writer::apply_edit(&mut derived, &ProjectEdit::Batch(edits));
    Some(derived)
}

// ============================================================================
// STAGE 17: NEW PROJECT CREATION WIZARD & MINISERVER HARDWARE MODEL CONVERSION
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MiniserverModel {
    #[default]
    Gen2,
    Compact,
    Go,
    Gen1,
}

impl MiniserverModel {
    pub const ALL: [MiniserverModel; 4] = [Self::Gen2, Self::Compact, Self::Go, Self::Gen1];

    pub fn code(self) -> &'static str {
        match self {
            Self::Gen2 => "gen2",
            Self::Compact => "compact",
            Self::Go => "go",
            Self::Gen1 => "gen1",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gen2" | "2" | "miniserver_gen2" | "loxlive_2" => Some(Self::Gen2),
            "compact" | "3" | "miniserver_compact" | "loxlive_3" => Some(Self::Compact),
            "go" | "1" | "miniserver_go" | "loxlive_1" => Some(Self::Go),
            "gen1" | "0" | "miniserver_gen1" | "loxlive_0" => Some(Self::Gen1),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Gen2 => "Miniserver Gen 2 (8x DI, 4x AI, 8x DO, Tree + Link)",
            Self::Compact => "Miniserver Compact (4x DI, Tree + Air + Link + Audio)",
            Self::Go => "Miniserver Go Gen 2 (Air + Link)",
            Self::Gen1 => "Miniserver Gen 1 (8x DI, 4x AI, 8x DO, 4x AQ, Link)",
        }
    }

    pub fn miniserver_attr(self) -> &'static str {
        match self {
            Self::Gen2 => "2",
            Self::Compact => "3",
            Self::Go => "1",
            Self::Gen1 => "0",
        }
    }

    pub fn tid(self) -> &'static str {
        match self {
            Self::Gen2 => "LoxLIVE_2",
            Self::Compact => "LoxLIVE_3",
            Self::Go => "LoxLIVE_1",
            Self::Gen1 => "LoxLIVE_0",
        }
    }

    pub fn digital_in_count(self) -> usize {
        match self {
            Self::Gen2 | Self::Gen1 => 8,
            Self::Compact => 4,
            Self::Go => 0,
        }
    }

    pub fn analog_in_count(self) -> usize {
        match self {
            Self::Gen2 | Self::Gen1 => 4,
            Self::Compact | Self::Go => 0,
        }
    }

    pub fn digital_out_count(self) -> usize {
        match self {
            Self::Gen2 | Self::Gen1 => 8,
            Self::Compact | Self::Go => 0,
        }
    }

    pub fn analog_out_count(self) -> usize {
        match self {
            Self::Gen1 => 4,
            Self::Gen2 | Self::Compact | Self::Go => 0,
        }
    }

    pub fn has_onboard_tree(self) -> bool {
        matches!(self, Self::Gen2 | Self::Compact)
    }

    pub fn has_onboard_air(self) -> bool {
        matches!(self, Self::Compact | Self::Go)
    }
}

pub fn detect_miniserver_model(project: &LoxoneProject) -> MiniserverModel {
    if let Some(ms) = find_first_by_type(project, "LoxLIVE") {
        if let Some(m) = ms.attr("Miniserver").and_then(MiniserverModel::from_code) {
            return m;
        }
        if let Some(tid) = ms.attr("TId").and_then(MiniserverModel::from_code) {
            return tid;
        }
    }
    MiniserverModel::Gen2
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProjectRoomSpec {
    pub title: String,
    pub sqm: u32,
    pub create_page: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProjectSpec {
    pub title: String,
    pub miniserver_model: MiniserverModel,
    pub miniserver_ip: String,
    pub miniserver_serial: String,
    pub creator: String,
    pub street: String,
    pub town: String,
    pub postcode: String,
    pub country: String,
    pub rooms: Vec<NewProjectRoomSpec>,
}

impl Default for NewProjectSpec {
    fn default() -> Self {
        Self {
            title: "New Smart Home".to_string(),
            miniserver_model: MiniserverModel::Gen2,
            miniserver_ip: "192.168.1.77".to_string(),
            miniserver_serial: "504F94FF0001".to_string(),
            creator: "Loxone Portable".to_string(),
            street: "Main Street 1".to_string(),
            town: "Warsaw".to_string(),
            postcode: "00-001".to_string(),
            country: "PL".to_string(),
            rooms: vec![
                NewProjectRoomSpec {
                    title: "Central Functions".to_string(),
                    sqm: 0,
                    create_page: true,
                },
                NewProjectRoomSpec {
                    title: "Living Room".to_string(),
                    sqm: 32,
                    create_page: true,
                },
                NewProjectRoomSpec {
                    title: "Kitchen".to_string(),
                    sqm: 16,
                    create_page: true,
                },
                NewProjectRoomSpec {
                    title: "Master Bedroom".to_string(),
                    sqm: 20,
                    create_page: true,
                },
                NewProjectRoomSpec {
                    title: "Bathroom".to_string(),
                    sqm: 10,
                    create_page: true,
                },
                NewProjectRoomSpec {
                    title: "Technical Room".to_string(),
                    sqm: 8,
                    create_page: true,
                },
            ],
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Creates a brand-new Loxone Config 17-compatible `.Loxone` project from scratch.
pub fn create_new_project(spec: &NewProjectSpec) -> Result<LoxoneProject, crate::parser::LoxoneError> {
    let doc_u = generate_loxone_uuid();
    let title = if spec.title.trim().is_empty() {
        "New Project"
    } else {
        spec.title.trim()
    };
    let ip = if spec.miniserver_ip.trim().is_empty() {
        "192.168.1.77"
    } else {
        spec.miniserver_ip.trim()
    };
    let serial = if spec.miniserver_serial.trim().is_empty() {
        "504F94FF0001"
    } else {
        spec.miniserver_serial.trim()
    };

    let cat_cap_u = generate_loxone_uuid();
    let cat_unassigned_u = generate_loxone_uuid();
    let cat_lighting_u = generate_loxone_uuid();
    let cat_shading_u = generate_loxone_uuid();
    let cat_climate_u = generate_loxone_uuid();
    let cat_audio_u = generate_loxone_uuid();
    let cat_security_u = generate_loxone_uuid();
    let cat_energy_u = generate_loxone_uuid();
    let cat_central_u = generate_loxone_uuid();

    let place_cap_u = generate_loxone_uuid();
    let place_unassigned_u = generate_loxone_uuid();
    let mut room_entries: Vec<(String, String, u32, bool)> = Vec::new();
    for r in &spec.rooms {
        if !r.title.trim().is_empty() {
            room_entries.push((
                generate_loxone_uuid(),
                r.title.trim().to_string(),
                r.sqm,
                r.create_page,
            ));
        }
    }

    let mut place_nds = String::new();
    let mut place_nodes = String::new();
    place_nds.push_str(&format!(
        "\t\t\t<nd u=\"{}\" t=\"1\" ctrlUuid=\"{}\"/>\r\n",
        generate_loxone_uuid(),
        place_unassigned_u
    ));
    place_nodes.push_str(&format!(
        "\t\t\t<C Type=\"Place\" V=\"178\" U=\"{}\" Title=\"Unassigned\" WF=\"6307840\" Icon=\"0000015c-00ff-0000-0000000000000000\" First=\"true\" PType=\"2\"/>\r\n",
        place_unassigned_u
    ));
    for (ru, rtitle, sqm, _) in &room_entries {
        place_nds.push_str(&format!(
            "\t\t\t<nd u=\"{}\" t=\"1\" ctrlUuid=\"{}\"/>\r\n",
            generate_loxone_uuid(),
            ru
        ));
        place_nodes.push_str(&format!(
            "\t\t\t<C Type=\"Place\" V=\"178\" U=\"{}\" Title=\"{}\" WF=\"16384\" Icon=\"00000220-00ff-0000-0000000000000000\" PType=\"2\" Sqm=\"{}\"/>\r\n",
            ru,
            xml_escape(rtitle),
            sqm
        ));
    }

    let model = spec.miniserver_model;
    let mut ms_children = String::new();

    if model.digital_in_count() > 0 {
        let cap_u = generate_loxone_uuid();
        ms_children.push_str(&format!(
            "\t\t\t<C Type=\"InputCaption\" V=\"178\" U=\"{cap_u}\" Title=\"Digital Inputs\" WF=\"16384\">\r\n"
        ));
        for idx in 1..=model.digital_in_count() {
            let di_u = generate_loxone_uuid();
            let q_u = generate_loxone_uuid();
            let qe_u = generate_loxone_uuid();
            ms_children.push_str(&format!(
                "\t\t\t\t<C Type=\"DigitalIn\" IName=\"I{idx}\" V=\"178\" U=\"{di_u}\" Title=\"Digital Input I{idx}\" Nio=\"2\" WF=\"16384\" MinTime=\"10\" Idx=\"{idx}\">\r\n\
\t\t\t\t\t<Co K=\"Q\" U=\"{q_u}\"/>\r\n\
\t\t\t\t\t<Co K=\"Qe\" U=\"{qe_u}\"/>\r\n\
\t\t\t\t\t<IoData Cr=\"{cat_unassigned_u}\" Pr=\"{place_unassigned_u}\" St=\"2\"/>\r\n\
\t\t\t\t\t<Display Unit=\"&lt;v&gt;\" StateOnly=\"true\"/>\r\n\
\t\t\t\t</C>\r\n"
            ));
        }
        ms_children.push_str("\t\t\t</C>\r\n");
    }

    if model.analog_in_count() > 0 {
        let cap_u = generate_loxone_uuid();
        ms_children.push_str(&format!(
            "\t\t\t<C Type=\"AnalogInputCaption\" V=\"178\" U=\"{cap_u}\" Title=\"Analog Inputs\" WF=\"16384\">\r\n"
        ));
        for idx in 1..=model.analog_in_count() {
            let ai_u = generate_loxone_uuid();
            let aq_u = generate_loxone_uuid();
            let q_u = generate_loxone_uuid();
            ms_children.push_str(&format!(
                "\t\t\t\t<C Type=\"VoltageIn\" IName=\"AI{idx}\" V=\"178\" U=\"{ai_u}\" Title=\"Analog Input AI{idx}\" Nio=\"2\" WF=\"16384\" Analog=\"true\" SourceValHigh=\"10\" DestValHigh=\"10\" MaxVal=\"10\" MinChange=\"0.1\" MinTime=\"10\" Idx=\"{idx}\">\r\n\
\t\t\t\t\t<Co K=\"AQ\" U=\"{aq_u}\"/>\r\n\
\t\t\t\t\t<Co K=\"Q\" U=\"{q_u}\"/>\r\n\
\t\t\t\t\t<IoData Cr=\"{cat_unassigned_u}\" Pr=\"{place_unassigned_u}\" St=\"2\"/>\r\n\
\t\t\t\t\t<Display Type=\"1\" Unit=\"&lt;v.1&gt;\" StateOnly=\"true\"/>\r\n\
\t\t\t\t</C>\r\n"
            ));
        }
        ms_children.push_str("\t\t\t</C>\r\n");
    }

    if model.digital_out_count() > 0 {
        let cap_u = generate_loxone_uuid();
        ms_children.push_str(&format!(
            "\t\t\t<C Type=\"OutputCaption\" V=\"178\" U=\"{cap_u}\" Title=\"Digital Outputs\" WF=\"16384\">\r\n"
        ));
        for idx in 1..=model.digital_out_count() {
            let do_u = generate_loxone_uuid();
            let i_u = generate_loxone_uuid();
            ms_children.push_str(&format!(
                "\t\t\t\t<C Type=\"Actor\" IName=\"Q{idx}\" V=\"178\" U=\"{do_u}\" Title=\"Relay Output Q{idx}\" Nio=\"1\" WF=\"16384\" Idx=\"{idx}\">\r\n\
\t\t\t\t\t<Co K=\"I\" U=\"{i_u}\"/>\r\n\
\t\t\t\t\t<IoData Cr=\"{cat_unassigned_u}\" Pr=\"{place_unassigned_u}\" St=\"2\"/>\r\n\
\t\t\t\t\t<Display Unit=\"&lt;v.1&gt;\" StateOnly=\"true\"/>\r\n\
\t\t\t\t</C>\r\n"
            ));
        }
        ms_children.push_str("\t\t\t</C>\r\n");
    }

    if model.analog_out_count() > 0 {
        let cap_u = generate_loxone_uuid();
        ms_children.push_str(&format!(
            "\t\t\t<C Type=\"AnalogOutputCaption\" V=\"178\" U=\"{cap_u}\" Title=\"Analog Outputs\" WF=\"16384\">\r\n"
        ));
        for idx in 1..=model.analog_out_count() {
            let ao_u = generate_loxone_uuid();
            let i_u = generate_loxone_uuid();
            ms_children.push_str(&format!(
                "\t\t\t\t<C Type=\"AnalogActor\" IName=\"AQ{idx}\" V=\"178\" U=\"{ao_u}\" Title=\"Analog Output AQ{idx}\" Nio=\"1\" WF=\"16384\" Idx=\"{idx}\">\r\n\
\t\t\t\t\t<Co K=\"I\" U=\"{i_u}\"/>\r\n\
\t\t\t\t\t<IoData Cr=\"{cat_unassigned_u}\" Pr=\"{place_unassigned_u}\" St=\"2\"/>\r\n\
\t\t\t\t\t<Display Unit=\"&lt;v.1&gt;V\" StateOnly=\"true\"/>\r\n\
\t\t\t\t</C>\r\n"
            ));
        }
        ms_children.push_str("\t\t\t</C>\r\n");
    }

    ms_children.push_str(&format!(
        "\t\t\t<C Type=\"VirtualInCaption\" V=\"178\" U=\"{}\" Title=\"Virtual Inputs\" WF=\"16384\"/>\r\n\
\t\t\t<C Type=\"VirtualOutCaption\" V=\"178\" U=\"{}\" Title=\"Virtual Outputs\" WF=\"16384\"/>\r\n",
        generate_loxone_uuid(),
        generate_loxone_uuid()
    ));

    if model.has_onboard_tree() {
        ms_children.push_str(&format!(
            "\t\t\t<C Type=\"LoxTree\" TId=\"LoxCaption_2\" V=\"178\" U=\"{}\" Title=\"Tree\" WF=\"16384\" IDev=\"true\" Serial=\"00000000\">\r\n\
\t\t\t\t<IoData/>\r\n\
\t\t\t\t<C Type=\"LoxCaption\" V=\"178\" U=\"{}\" Title=\"Tree Branch\" Cl=\"0,0,0\" WF=\"16384\" CaptionType=\"3\" SubType=\"3\"/>\r\n\
\t\t\t</C>\r\n",
            generate_loxone_uuid(),
            generate_loxone_uuid()
        ));
    }

    if model.has_onboard_air() {
        ms_children.push_str(&format!(
            "\t\t\t<C Type=\"LoxAIR\" TId=\"LoxAIR\" V=\"178\" U=\"{}\" Title=\"Air\" WF=\"16384\" IDev=\"true\" Serial=\"00000000\">\r\n\
\t\t\t\t<IoData/>\r\n\
\t\t\t</C>\r\n",
            generate_loxone_uuid()
        ));
    }

    let mut page_nodes = String::new();
    for (ru, rtitle, _, create_page) in &room_entries {
        if *create_page {
            let pu = generate_loxone_uuid();
            page_nodes.push_str(&format!(
                "\t\t\t<C Type=\"Page\" V=\"178\" U=\"{pu}\" Title=\"{}\" WF=\"16384\" Color=\"83,110,123\">\r\n\
\t\t\t\t<IoData Cr=\"{cat_central_u}\" Pr=\"{ru}\"/>\r\n\
\t\t\t</C>\r\n",
                xml_escape(rtitle)
            ));
        }
    }
    if page_nodes.is_empty() {
        let pu = generate_loxone_uuid();
        page_nodes.push_str(&format!(
            "\t\t\t<C Type=\"Page\" V=\"178\" U=\"{pu}\" Title=\"Overview\" WF=\"16384\" Color=\"83,110,123\">\r\n\
\t\t\t\t<IoData Cr=\"{cat_central_u}\" Pr=\"{place_unassigned_u}\"/>\r\n\
\t\t\t</C>\r\n"
        ));
    }

    let ms_u = generate_loxone_uuid();
    let user_cap_outer = generate_loxone_uuid();
    let user_cap_u = generate_loxone_uuid();
    let admin_u = generate_loxone_uuid();
    let perm_cap_u = generate_loxone_uuid();
    let perm_cfg_u = generate_loxone_uuid();
    let perm_web_u = generate_loxone_uuid();
    let grp_cap_u = generate_loxone_uuid();
    let grp_admin_u = generate_loxone_uuid();
    let tmr_cap_u = generate_loxone_uuid();
    let tmr_always_u = generate_loxone_uuid();

    let xml = format!(
        "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\
<ControlList Version=\"274\" LxAV=\"87\" NextObj=\"1000\" NextConst=\"1\" NextNote=\"1\" NextMem=\"1\">\r\n\
\t<C Type=\"Document\" V=\"17020828\" U=\"{doc_u}\" Title=\"{}\" ConfigVersion=\"17020828\" PaperFormat=\"2\" SPSfreq=\"100\" OpenIsHigh=\"false\" Date=\"2026-11-10 12:00:00\" CDate=\"2026-11-10 12:00:00\" DateS=\"559780177\" Street=\"{}\" Town=\"{}\" PostCode=\"{}\" Ctry=\"{}\" Latitude=\"52.2297\" Longitude=\"21.0122\" SL=\"1\" Timezone=\"Europe/Warsaw\" Creator=\"{}\" Currency=\"EUR\" APPID=\"0000\">\r\n\
\t\t<C Type=\"CategoryCaption\" V=\"178\" U=\"{cat_cap_u}\" Title=\"Categories\" WF=\"16384\">\r\n\
\t\t\t<C Type=\"Category\" V=\"178\" U=\"{cat_unassigned_u}\" Title=\"Unassigned\" WF=\"6307840\" Icon=\"00000038-00ff-0000-0000000000000000\" First=\"true\" PType=\"2\"/>\r\n\
\t\t\t<C Type=\"Category\" V=\"178\" U=\"{cat_lighting_u}\" Title=\"Lighting\" WF=\"16384\" Icon=\"00000153-00ff-0000-0000000000000000\" CatGroup=\"2\" PType=\"2\"/>\r\n\
\t\t\t<C Type=\"Category\" V=\"178\" U=\"{cat_shading_u}\" Title=\"Shading\" WF=\"16384\" Icon=\"0000003b-00ff-0000-0000000000000000\" CatGroup=\"1\" PType=\"2\"/>\r\n\
\t\t\t<C Type=\"Category\" V=\"178\" U=\"{cat_climate_u}\" Title=\"Climate\" WF=\"16384\" Icon=\"00000246-00ff-0000-0000000000000000\" CatGroup=\"5\" PType=\"2\"/>\r\n\
\t\t\t<C Type=\"Category\" V=\"178\" U=\"{cat_audio_u}\" Title=\"Audio\" WF=\"16384\" Icon=\"0000016f-00ff-0000-0000000000000000\" CatGroup=\"4\" PType=\"2\"/>\r\n\
\t\t\t<C Type=\"Category\" V=\"178\" U=\"{cat_security_u}\" Title=\"Security\" WF=\"16384\" Icon=\"0000004e-00ff-0000-0000000000000000\" CatGroup=\"12\" PType=\"2\"/>\r\n\
\t\t\t<C Type=\"Category\" V=\"178\" U=\"{cat_energy_u}\" Title=\"Energy\" WF=\"16384\" Icon=\"00000107-00ff-0000-0000000000000000\" CatGroup=\"51\" PType=\"2\"/>\r\n\
\t\t\t<C Type=\"Category\" V=\"178\" U=\"{cat_central_u}\" Title=\"Central\" WF=\"16384\" Icon=\"00000069-00ff-0000-0000000000000000\" CatGroup=\"52\" PType=\"2\"/>\r\n\
\t\t</C>\r\n\
\t\t<C Type=\"PlaceCaption\" V=\"178\" U=\"{place_cap_u}\" Title=\"Rooms\" WF=\"16384\" u=\"00000000-0000-0000-0000000000000000\">\r\n\
{place_nds}{place_nodes}\t\t</C>\r\n\
\t\t<C Type=\"LoxCaption\" V=\"178\" U=\"{user_cap_outer}\" Title=\"Users and Rights\" Cl=\"0,0,0\" WF=\"16384\" CaptionType=\"20\" SubType=\"20\">\r\n\
\t\t\t<C Type=\"UserCaption\" V=\"178\" U=\"{user_cap_u}\" Title=\"Users\" WF=\"16384\">\r\n\
\t\t\t\t<C Type=\"User\" V=\"178\" U=\"{admin_u}\" Title=\"admin\" WF=\"16384\" Main=\"true\" FiN=\"Admin\" LaN=\"User\" Ema=\"\" Phn=\"\"/>\r\n\
\t\t\t</C>\r\n\
\t\t\t<C Type=\"LoxCaption\" V=\"178\" U=\"{perm_cap_u}\" Title=\"Permissions\" Cl=\"0,0,0\" WF=\"16384\" CaptionType=\"12\" SubType=\"12\">\r\n\
\t\t\t\t<C Type=\"Permission\" V=\"178\" U=\"{perm_cfg_u}\" Title=\"Loxone Config\" Cl=\"0,0,0\" WF=\"16384\" PT=\"1\"/>\r\n\
\t\t\t\t<C Type=\"Permission\" V=\"178\" U=\"{perm_web_u}\" Title=\"Web / App\" Cl=\"0,0,0\" WF=\"16384\" PT=\"3\"/>\r\n\
\t\t\t</C>\r\n\
\t\t\t<C Type=\"UserGroupCaption\" V=\"178\" U=\"{grp_cap_u}\" Title=\"User Groups\" WF=\"16384\">\r\n\
\t\t\t\t<C Type=\"UserGroup\" V=\"178\" U=\"{grp_admin_u}\" Title=\"Administrators\" WF=\"16384\" Rights=\"-1\" UgType=\"4\" User=\"{admin_u}\" TMR=\"{tmr_always_u}\"/>\r\n\
\t\t\t</C>\r\n\
\t\t\t<C Type=\"LoxCaption\" V=\"178\" U=\"{tmr_cap_u}\" Title=\"Access Schedules\" Cl=\"0,0,0\" WF=\"16384\" CaptionType=\"11\" SubType=\"11\">\r\n\
\t\t\t\t<C Type=\"SwitchingTimer\" V=\"178\" U=\"{tmr_always_u}\" Title=\"Always\" Cl=\"0,0,0\" WF=\"16384\" M=\"3\" FIX=\"1\"/>\r\n\
\t\t\t</C>\r\n\
\t\t</C>\r\n\
\t\t<C Type=\"ConstantCaption\" V=\"178\" U=\"{}\" Title=\"Constants\" WF=\"16384\"/>\r\n\
\t\t<C Type=\"MemoryCaption\" V=\"178\" U=\"{}\" Title=\"Flags\" WF=\"16384\"/>\r\n\
\t\t<C Type=\"ModeCaption\" V=\"178\" U=\"{}\" Title=\"Operating Modes\" WF=\"16384\">\r\n\
\t\t\t<C Type=\"Mode\" V=\"178\" U=\"00000000-0000-0001-1500000000000000\" Title=\"Holiday\" Nio=\"2\" WF=\"16384\"/>\r\n\
\t\t\t<C Type=\"Mode\" V=\"178\" U=\"00000000-0000-0002-1500000000000000\" Title=\"Vacation\" Nio=\"2\" WF=\"16384\" Mode=\"1\"/>\r\n\
\t\t</C>\r\n\
\t\t<C Type=\"CalendarCaption\" V=\"178\" U=\"{}\" Title=\"Operating Times\" WF=\"16384\"/>\r\n\
\t\t<C Type=\"LoxCaption\" V=\"178\" U=\"{}\" Title=\"Network Devices\" Cl=\"0,0,0\" WF=\"16384\" CaptionType=\"6\" SubType=\"6\"/>\r\n\
\t\t<C Type=\"AutoPilot\" V=\"178\" U=\"{}\" Title=\"Autopilot Rules\" Cl=\"0,0,0\" Nio=\"2\" WF=\"20480\"/>\r\n\
\t\t<C Type=\"LoxLIVE\" TId=\"{}\" V=\"178\" U=\"{ms_u}\" Title=\"{}\" WF=\"16400\" Miniserver=\"{}\" ExtAddr=\"{}\" IntAddr=\"{}\" ExP=\"80\" ExP2=\"443\" Serial=\"{}\" HoN=\"miniserver\">\r\n\
\t\t\t<IoData/>\r\n\
{ms_children}\t\t</C>\r\n\
\t\t<C Type=\"Program\" V=\"178\" U=\"{}\" Title=\"Program\" WF=\"16384\">\r\n\
{page_nodes}\t\t</C>\r\n\
\t</C>\r\n\
</ControlList>\r\n",
        xml_escape(title),
        xml_escape(&spec.street),
        xml_escape(&spec.town),
        xml_escape(&spec.postcode),
        xml_escape(&spec.country),
        xml_escape(&spec.creator),
        generate_loxone_uuid(),
        generate_loxone_uuid(),
        generate_loxone_uuid(),
        generate_loxone_uuid(),
        generate_loxone_uuid(),
        generate_loxone_uuid(),
        model.tid(),
        xml_escape(title),
        model.miniserver_attr(),
        xml_escape(ip),
        xml_escape(ip),
        xml_escape(&serial.to_ascii_uppercase()),
        generate_loxone_uuid(),
    );

    crate::parser::parse_loxone_xml(&xml)
}

/// Builds a `ProjectEdit` that converts the primary `LoxLIVE` Miniserver in `project` to `target_model`,
/// preserving existing wired channels and adding any missing onboard I/O channels or buses.
pub fn build_convert_miniserver_model_edit(
    project: &LoxoneProject,
    target_model: MiniserverModel,
) -> Option<ProjectEdit> {
    let ms = find_first_by_type(project, "LoxLIVE")?;
    let ms_uuid = ms.uuid.clone();
    let mut edits = vec![
        ProjectEdit::SetControlAttr {
            control_uuid: ms_uuid.clone(),
            attr_name: "Miniserver".to_string(),
            value: Some(target_model.miniserver_attr().to_string()),
        },
        ProjectEdit::SetControlAttr {
            control_uuid: ms_uuid.clone(),
            attr_name: "TId".to_string(),
            value: Some(target_model.tid().to_string()),
        },
    ];

    // Ensure AnalogOutputCaption (AQ1..AQ4) if converting to Gen1
    if target_model.analog_out_count() > 0
        && !ms
            .children
            .iter()
            .any(|c| c.control_type == "AnalogOutputCaption")
    {
        let mut ao_children = Vec::new();
        for idx in 1..=target_model.analog_out_count() {
            let mut ch = new_tree_node("AnalogActor", &format!("Analog Output AQ{idx}"));
            ch.raw_attributes
                .push(("IName".to_string(), format!("AQ{idx}")));
            ch.raw_attributes
                .push(("Idx".to_string(), idx.to_string()));
            ch.pins.push(ConnectorPin {
                uuid: generate_loxone_uuid(),
                key: "I".to_string(),
                default_value: None,
                inverted: false,
                pin_kind: PinKind::Input,
                incoming_links: Vec::new(),
            });
            ao_children.push(ch);
        }
        let mut cap = new_tree_node("AnalogOutputCaption", "Analog Outputs");
        cap.children = ao_children;
        edits.push(ProjectEdit::AddTreeNode {
            section: TreeSectionKind::Periphery,
            parent_uuid: Some(ms_uuid.clone()),
            node: cap,
        });
    }

    // Ensure onboard LoxAIR if converting to Compact or Go
    if target_model.has_onboard_air() && !ms.children.iter().any(|c| c.control_type == "LoxAIR") {
        let mut air = new_tree_node("LoxAIR", "Air");
        air.raw_attributes
            .push(("TId".to_string(), "LoxAIR".to_string()));
        air.raw_attributes
            .push(("IDev".to_string(), "true".to_string()));
        air.raw_attributes
            .push(("Serial".to_string(), "00000000".to_string()));
        edits.push(ProjectEdit::AddTreeNode {
            section: TreeSectionKind::Periphery,
            parent_uuid: Some(ms_uuid.clone()),
            node: air,
        });
    }

    // Ensure onboard LoxTree if converting to Gen2 or Compact
    if target_model.has_onboard_tree() && !ms.children.iter().any(|c| c.control_type == "LoxTree") {
        let mut branch = new_tree_node("LoxCaption", "Tree Branch");
        branch
            .raw_attributes
            .push(("CaptionType".to_string(), "3".to_string()));
        branch
            .raw_attributes
            .push(("SubType".to_string(), "3".to_string()));
        let mut tree = new_tree_node("LoxTree", "Tree");
        tree.raw_attributes
            .push(("TId".to_string(), "LoxCaption_2".to_string()));
        tree.raw_attributes
            .push(("IDev".to_string(), "true".to_string()));
        tree.raw_attributes
            .push(("Serial".to_string(), "00000000".to_string()));
        tree.children.push(branch);
        edits.push(ProjectEdit::AddTreeNode {
            section: TreeSectionKind::Periphery,
            parent_uuid: Some(ms_uuid),
            node: tree,
        });
    }

    Some(ProjectEdit::Batch(edits))
}

// ============================================================================
// STAGE 19: CUSTOM BLOCK TEMPLATES & REUSABLE SCHEMATIC MACROS
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct SchematicMacroBlock {
    pub rel_id: String,
    pub control_type: String,
    pub title: String,
    pub offset_x: f32,
    pub offset_y: f32,
    pub attributes: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchematicMacroWire {
    pub from_block_rel_id: String,
    pub from_pin_key: String,
    pub to_block_rel_id: String,
    pub to_pin_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchematicMacroTemplate {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub blocks: Vec<SchematicMacroBlock>,
    pub wires: Vec<SchematicMacroWire>,
}

pub fn builtin_schematic_macros() -> Vec<SchematicMacroTemplate> {
    vec![
        SchematicMacroTemplate {
            id: "lighting_motion_room".to_string(),
            name: "Room Lighting + Presence + Switch".to_string(),
            category: "Lighting".to_string(),
            description: "Pushbutton + Presence Detector wired into a LightControllerV2".to_string(),
            blocks: vec![
                SchematicMacroBlock {
                    rel_id: "b0".to_string(),
                    control_type: "PushButton".to_string(),
                    title: "Wall Switch".to_string(),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    attributes: Vec::new(),
                },
                SchematicMacroBlock {
                    rel_id: "b1".to_string(),
                    control_type: "PresenceDetector".to_string(),
                    title: "Room Presence".to_string(),
                    offset_x: 0.0,
                    offset_y: 192.0,
                    attributes: Vec::new(),
                },
                SchematicMacroBlock {
                    rel_id: "b2".to_string(),
                    control_type: "LightControllerV2".to_string(),
                    title: "Lighting Controller".to_string(),
                    offset_x: 320.0,
                    offset_y: 32.0,
                    attributes: Vec::new(),
                },
            ],
            wires: vec![
                SchematicMacroWire {
                    from_block_rel_id: "b0".to_string(),
                    from_pin_key: "O".to_string(),
                    to_block_rel_id: "b2".to_string(),
                    to_pin_key: "Tr".to_string(),
                },
                SchematicMacroWire {
                    from_block_rel_id: "b1".to_string(),
                    from_pin_key: "Q".to_string(),
                    to_block_rel_id: "b2".to_string(),
                    to_pin_key: "M".to_string(),
                },
            ],
        },
        SchematicMacroTemplate {
            id: "shading_wind_sun_safety".to_string(),
            name: "Automatic Shading + Safety Lock".to_string(),
            category: "Shading".to_string(),
            description: "Up/Down switch + Wind/Frost AND safety lock wired into AutomaticJalousie".to_string(),
            blocks: vec![
                SchematicMacroBlock {
                    rel_id: "b0".to_string(),
                    control_type: "PushButton".to_string(),
                    title: "Blind Switch".to_string(),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    attributes: Vec::new(),
                },
                SchematicMacroBlock {
                    rel_id: "b1".to_string(),
                    control_type: "And".to_string(),
                    title: "Wind & Frost Safety".to_string(),
                    offset_x: 0.0,
                    offset_y: 192.0,
                    attributes: Vec::new(),
                },
                SchematicMacroBlock {
                    rel_id: "b2".to_string(),
                    control_type: "AutomaticJalousie".to_string(),
                    title: "Blind Controller".to_string(),
                    offset_x: 320.0,
                    offset_y: 32.0,
                    attributes: Vec::new(),
                },
            ],
            wires: vec![
                SchematicMacroWire {
                    from_block_rel_id: "b0".to_string(),
                    from_pin_key: "O".to_string(),
                    to_block_rel_id: "b2".to_string(),
                    to_pin_key: "Au".to_string(),
                },
                SchematicMacroWire {
                    from_block_rel_id: "b1".to_string(),
                    from_pin_key: "Q".to_string(),
                    to_block_rel_id: "b2".to_string(),
                    to_pin_key: "Sp".to_string(),
                },
            ],
        },
        SchematicMacroTemplate {
            id: "hvac_room_climate".to_string(),
            name: "Intelligent Room Climate + Window Stop".to_string(),
            category: "Climate".to_string(),
            description: "Intelligent Room Controller v2 with Window Open contact inhibit".to_string(),
            blocks: vec![
                SchematicMacroBlock {
                    rel_id: "b0".to_string(),
                    control_type: "PushButton".to_string(),
                    title: "Window Contact".to_string(),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    attributes: Vec::new(),
                },
                SchematicMacroBlock {
                    rel_id: "b1".to_string(),
                    control_type: "HeatIRoomController2".to_string(),
                    title: "Room Climate Controller".to_string(),
                    offset_x: 320.0,
                    offset_y: 0.0,
                    attributes: Vec::new(),
                },
            ],
            wires: vec![SchematicMacroWire {
                from_block_rel_id: "b0".to_string(),
                from_pin_key: "O".to_string(),
                to_block_rel_id: "b1".to_string(),
                to_pin_key: "Wp".to_string(),
            }],
        },
    ]
}

/// Extracts a reusable `SchematicMacroTemplate` from selected blocks on `page` (or all blocks on `page`
/// when `block_uuids` is empty).
pub fn extract_macro_from_page(
    page: &crate::model::Page,
    block_uuids: &[String],
    id: &str,
    name: &str,
    category: &str,
    description: &str,
) -> Option<SchematicMacroTemplate> {
    let selected: Vec<&crate::model::PageBlock> = page
        .blocks
        .iter()
        .filter(|b| {
            b.has_position
                && (block_uuids.is_empty() || block_uuids.iter().any(|u| u == &b.uuid))
        })
        .collect();
    if selected.is_empty() {
        return None;
    }

    let min_x = selected
        .iter()
        .map(|b| b.px)
        .fold(f32::INFINITY, f32::min);
    let min_y = selected
        .iter()
        .map(|b| b.py)
        .fold(f32::INFINITY, f32::min);

    let mut uuid_to_rel: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    let mut out_pin_to_rel: std::collections::HashMap<&str, (String, String)> =
        std::collections::HashMap::new();
    let mut macro_blocks = Vec::new();

    for (idx, blk) in selected.iter().enumerate() {
        let rel_id = format!("b{idx}");
        uuid_to_rel.insert(blk.uuid.as_str(), rel_id.clone());
        for pin in &blk.pins {
            if !pin.uuid.is_empty() {
                out_pin_to_rel.insert(pin.uuid.as_str(), (rel_id.clone(), pin.key.clone()));
            }
        }
        let attrs: Vec<(String, String)> = blk
            .raw_attributes
            .iter()
            .filter(|(k, _)| {
                !matches!(
                    k.as_str(),
                    "Type" | "U" | "V" | "WF" | "Title" | "Px" | "Py" | "Px2" | "Py2"
                )
            })
            .cloned()
            .collect();
        macro_blocks.push(SchematicMacroBlock {
            rel_id,
            control_type: blk.block_type.clone(),
            title: blk.title.clone(),
            offset_x: (blk.px - min_x).max(0.0),
            offset_y: (blk.py - min_y).max(0.0),
            attributes: attrs,
        });
    }

    let mut macro_wires = Vec::new();
    for blk in &selected {
        let Some(to_rel) = uuid_to_rel.get(blk.uuid.as_str()) else {
            continue;
        };
        for pin in &blk.pins {
            for link in &pin.incoming_links {
                if let Some((from_rel, from_key)) = out_pin_to_rel.get(link.source_pin_uuid.as_str())
                {
                    macro_wires.push(SchematicMacroWire {
                        from_block_rel_id: from_rel.clone(),
                        from_pin_key: from_key.clone(),
                        to_block_rel_id: to_rel.clone(),
                        to_pin_key: pin.key.clone(),
                    });
                }
            }
        }
    }

    Some(SchematicMacroTemplate {
        id: if id.trim().is_empty() {
            format!("macro_{}", generate_loxone_uuid().split('-').next().unwrap_or("1"))
        } else {
            id.trim().to_string()
        },
        name: if name.trim().is_empty() {
            format!("{} Macro", page.title)
        } else {
            name.trim().to_string()
        },
        category: if category.trim().is_empty() {
            "Custom".to_string()
        } else {
            category.trim().to_string()
        },
        description: description.trim().to_string(),
        blocks: macro_blocks,
        wires: macro_wires,
    })
}

/// Instantiates `macro_tpl` onto `page_uuid` at `(origin_x, origin_y)`, returning the newly created
/// block UUIDs and the batch `ProjectEdit`.
pub fn build_instantiate_macro_edits(
    project: &LoxoneProject,
    page_uuid: &str,
    macro_tpl: &SchematicMacroTemplate,
    origin_x: f32,
    origin_y: f32,
    title_prefix: Option<&str>,
) -> Result<(Vec<String>, ProjectEdit), String> {
    if !project.pages.iter().any(|p| p.uuid == page_uuid) {
        return Err(format!("Page '{page_uuid}' not found in project"));
    }

    let mut edits = Vec::new();
    let mut created_uuids = Vec::new();
    let mut rel_blocks: std::collections::HashMap<String, crate::model::PageBlock> =
        std::collections::HashMap::new();

    for mb in &macro_tpl.blocks {
        let title = match title_prefix.map(str::trim).filter(|s| !s.is_empty()) {
            Some(pfx) => format!("{pfx} {}", mb.title),
            None => mb.title.clone(),
        };
        let mut blk = crate::factory::new_page_block(
            &mb.control_type,
            &title,
            origin_x + mb.offset_x,
            origin_y + mb.offset_y,
        );
        for (k, v) in &mb.attributes {
            if let Some((_, existing)) = blk.raw_attributes.iter_mut().find(|(ak, _)| ak == k) {
                *existing = v.clone();
            } else {
                blk.raw_attributes.push((k.clone(), v.clone()));
            }
        }
        created_uuids.push(blk.uuid.clone());
        rel_blocks.insert(mb.rel_id.clone(), blk.clone());
        edits.push(ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.to_string(),
            block: blk,
        });
    }

    for mw in &macro_tpl.wires {
        let Some(src_blk) = rel_blocks.get(&mw.from_block_rel_id) else {
            continue;
        };
        let Some(dst_blk) = rel_blocks.get(&mw.to_block_rel_id) else {
            continue;
        };
        let src_pin_uuid = src_blk
            .pins
            .iter()
            .find(|p| p.key == mw.from_pin_key)
            .or_else(|| {
                src_blk
                    .pins
                    .iter()
                    .find(|p| p.pin_kind == PinKind::Output)
            })
            .map(|p| p.uuid.clone());
        if let Some(sp_uuid) = src_pin_uuid {
            edits.push(ProjectEdit::AddWireLink {
                target_block_uuid: dst_blk.uuid.clone(),
                target_pin_key: mw.to_pin_key.clone(),
                source_pin_uuid: sp_uuid,
            });
        }
    }

    Ok((created_uuids, ProjectEdit::Batch(edits)))
}

// ============================================================================
// STAGE 20: VIRTUAL HTTP / TCP / UDP INPUT & OUTPUT COMMAND EDITOR
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualOutCommandEntry {
    pub uuid: String,
    pub parent_vo_uuid: String,
    pub i_name: String,
    pub title: String,
    pub analog: bool,
    pub cmd_on: String,
    pub cmd_on_post: String,
    pub cmd_on_method: String,
    pub cmd_off: String,
    pub cmd_off_post: String,
    pub repeat_sec: u32,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualOutEndpoint {
    pub uuid: String,
    pub i_name: String,
    pub title: String,
    pub address: String,
    pub close_after_send: bool,
    pub cmd_sep: String,
    pub room_uuid: Option<String>,
    pub commands: Vec<VirtualOutCommandEntry>,
}

pub fn collect_virtual_out_endpoints(project: &LoxoneProject) -> Vec<VirtualOutEndpoint> {
    let mut out = Vec::new();
    for vo in find_nodes_by_type(project, "VirtualOut") {
        let mut cmds = Vec::new();
        for ch in &vo.children {
            if ch.control_type == "VirtualOutCmd" {
                cmds.push(VirtualOutCommandEntry {
                    uuid: ch.uuid.clone(),
                    parent_vo_uuid: vo.uuid.clone(),
                    i_name: ch.attr("IName").unwrap_or_default().to_string(),
                    title: ch.title.clone(),
                    analog: ch
                        .attr("Analog")
                        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                        .unwrap_or(false),
                    cmd_on: ch.attr("CmdOn").unwrap_or_default().to_string(),
                    cmd_on_post: ch.attr("CmdOnPost").unwrap_or_default().to_string(),
                    cmd_on_method: ch.attr("CmdOnMethod").unwrap_or("0").to_string(),
                    cmd_off: ch.attr("CmdOff").unwrap_or_default().to_string(),
                    cmd_off_post: ch.attr("CmdOffPost").unwrap_or_default().to_string(),
                    repeat_sec: ch
                        .attr("Repeat")
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(0),
                    room_uuid: ch.room_uuid.clone(),
                    category_uuid: ch.category_uuid.clone(),
                });
            }
        }
        out.push(VirtualOutEndpoint {
            uuid: vo.uuid.clone(),
            i_name: vo.attr("IName").unwrap_or_default().to_string(),
            title: vo.title.clone(),
            address: vo.attr("Address").unwrap_or_default().to_string(),
            close_after_send: vo
                .attr("CloseAfterSend")
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(true),
            cmd_sep: vo.attr("CmdSep").unwrap_or(";").to_string(),
            room_uuid: vo.room_uuid.clone(),
            commands: cmds,
        });
    }
    out
}

pub fn build_upsert_virtual_out_edit(
    project: &LoxoneProject,
    endpoint: &VirtualOutEndpoint,
) -> Result<(String, ProjectEdit), String> {
    if endpoint.title.trim().is_empty() {
        return Err("VirtualOut title cannot be empty".to_string());
    }
    if endpoint.address.trim().is_empty() {
        return Err("VirtualOut address (http://, tcp://, udp://) cannot be empty".to_string());
    }

    let vo_uuid = if endpoint.uuid.trim().is_empty() {
        generate_loxone_uuid()
    } else {
        endpoint.uuid.trim().to_string()
    };

    if find_node_by_uuid(project, &vo_uuid).is_some() {
        let edits = vec![
            ProjectEdit::SetControlAttr {
                control_uuid: vo_uuid.clone(),
                attr_name: "Title".to_string(),
                value: Some(endpoint.title.trim().to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: vo_uuid.clone(),
                attr_name: "Address".to_string(),
                value: Some(endpoint.address.trim().to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: vo_uuid.clone(),
                attr_name: "CloseAfterSend".to_string(),
                value: Some(if endpoint.close_after_send { "true" } else { "false" }.to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: vo_uuid.clone(),
                attr_name: "CmdSep".to_string(),
                value: Some(endpoint.cmd_sep.clone()),
            },
        ];
        return Ok((vo_uuid, ProjectEdit::Batch(edits)));
    }

    let parent_cap_uuid = find_first_by_type(project, "VirtualOutCaption")
        .map(|n| n.uuid.clone())
        .or_else(|| find_first_by_type(project, "LoxLIVE").map(|n| n.uuid.clone()));

    let next_idx = find_nodes_by_type(project, "VirtualOut").len() + 1;
    let i_name = if endpoint.i_name.trim().is_empty() {
        format!("VQ{next_idx}")
    } else {
        endpoint.i_name.trim().to_string()
    };

    let mut node = new_tree_node("VirtualOut", endpoint.title.trim());
    node.uuid = vo_uuid.clone();
    node.section = TreeSectionKind::Periphery;
    node.room_uuid = endpoint.room_uuid.clone();
    node.raw_attributes = vec![
        ("Type".to_string(), "VirtualOut".to_string()),
        ("IName".to_string(), i_name),
        ("V".to_string(), "178".to_string()),
        ("U".to_string(), vo_uuid.clone()),
        ("Title".to_string(), endpoint.title.trim().to_string()),
        ("WF".to_string(), "16384".to_string()),
        ("Address".to_string(), endpoint.address.trim().to_string()),
        (
            "CloseAfterSend".to_string(),
            if endpoint.close_after_send {
                "true".to_string()
            } else {
                "false".to_string()
            },
        ),
        (
            "CmdSep".to_string(),
            if endpoint.cmd_sep.is_empty() {
                ";".to_string()
            } else {
                endpoint.cmd_sep.clone()
            },
        ),
    ];

    Ok((
        vo_uuid,
        ProjectEdit::AddTreeNode {
            section: TreeSectionKind::Periphery,
            parent_uuid: parent_cap_uuid,
            node,
        },
    ))
}

pub fn build_upsert_virtual_out_cmd_edit(
    project: &LoxoneProject,
    cmd: &VirtualOutCommandEntry,
) -> Result<(String, ProjectEdit), String> {
    if cmd.title.trim().is_empty() {
        return Err("VirtualOutCmd title cannot be empty".to_string());
    }
    if cmd.parent_vo_uuid.trim().is_empty() {
        return Err("VirtualOutCmd requires a parent VirtualOut UUID".to_string());
    }
    if find_node_by_uuid(project, cmd.parent_vo_uuid.trim()).is_none() {
        return Err(format!("Parent VirtualOut '{}' not found", cmd.parent_vo_uuid));
    }

    let cmd_uuid = if cmd.uuid.trim().is_empty() {
        generate_loxone_uuid()
    } else {
        cmd.uuid.trim().to_string()
    };

    if find_node_by_uuid(project, &cmd_uuid).is_some() {
        let edits = vec![
            ProjectEdit::SetControlAttr {
                control_uuid: cmd_uuid.clone(),
                attr_name: "Title".to_string(),
                value: Some(cmd.title.trim().to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: cmd_uuid.clone(),
                attr_name: "Analog".to_string(),
                value: Some(if cmd.analog { "true" } else { "false" }.to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: cmd_uuid.clone(),
                attr_name: "CmdOn".to_string(),
                value: Some(cmd.cmd_on.trim().to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: cmd_uuid.clone(),
                attr_name: "CmdOnPost".to_string(),
                value: (!cmd.cmd_on_post.trim().is_empty())
                    .then(|| cmd.cmd_on_post.trim().to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: cmd_uuid.clone(),
                attr_name: "CmdOnMethod".to_string(),
                value: Some(cmd.cmd_on_method.trim().to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: cmd_uuid.clone(),
                attr_name: "CmdOff".to_string(),
                value: (!cmd.cmd_off.trim().is_empty()).then(|| cmd.cmd_off.trim().to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: cmd_uuid.clone(),
                attr_name: "CmdOffPost".to_string(),
                value: (!cmd.cmd_off_post.trim().is_empty())
                    .then(|| cmd.cmd_off_post.trim().to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: cmd_uuid.clone(),
                attr_name: "Repeat".to_string(),
                value: (cmd.repeat_sec > 0).then(|| cmd.repeat_sec.to_string()),
            },
        ];
        return Ok((cmd_uuid, ProjectEdit::Batch(edits)));
    }

    let next_idx = find_nodes_by_type(project, "VirtualOutCmd").len() + 1;
    let i_name = if cmd.i_name.trim().is_empty() {
        format!("VQC{next_idx}")
    } else {
        cmd.i_name.trim().to_string()
    };

    let mut node = new_tree_node("VirtualOutCmd", cmd.title.trim());
    node.uuid = cmd_uuid.clone();
    node.section = TreeSectionKind::Periphery;
    node.room_uuid = cmd.room_uuid.clone();
    node.category_uuid = cmd.category_uuid.clone();
    node.pins.push(ConnectorPin {
        uuid: generate_loxone_uuid(),
        key: "I".to_string(),
        default_value: None,
        inverted: false,
        pin_kind: PinKind::Input,
        incoming_links: Vec::new(),
    });
    node.raw_attributes = vec![
        ("Type".to_string(), "VirtualOutCmd".to_string()),
        ("IName".to_string(), i_name),
        ("V".to_string(), "178".to_string()),
        ("U".to_string(), cmd_uuid.clone()),
        ("Title".to_string(), cmd.title.trim().to_string()),
        ("Nio".to_string(), "1".to_string()),
        ("WF".to_string(), "16400".to_string()),
        (
            "Analog".to_string(),
            if cmd.analog { "true" } else { "false" }.to_string(),
        ),
        ("CmdOn".to_string(), cmd.cmd_on.trim().to_string()),
        (
            "CmdOnMethod".to_string(),
            if cmd.cmd_on_method.trim().is_empty() {
                "0".to_string()
            } else {
                cmd.cmd_on_method.trim().to_string()
            },
        ),
        ("Tx".to_string(), "false".to_string()),
    ];
    if !cmd.cmd_on_post.trim().is_empty() {
        node.raw_attributes
            .push(("CmdOnPost".to_string(), cmd.cmd_on_post.trim().to_string()));
    }
    if !cmd.cmd_off.trim().is_empty() {
        node.raw_attributes
            .push(("CmdOff".to_string(), cmd.cmd_off.trim().to_string()));
    }
    if !cmd.cmd_off_post.trim().is_empty() {
        node.raw_attributes
            .push(("CmdOffPost".to_string(), cmd.cmd_off_post.trim().to_string()));
    }
    if cmd.repeat_sec > 0 {
        node.raw_attributes
            .push(("Repeat".to_string(), cmd.repeat_sec.to_string()));
    }

    Ok((
        cmd_uuid,
        ProjectEdit::AddTreeNode {
            section: TreeSectionKind::Periphery,
            parent_uuid: Some(cmd.parent_vo_uuid.trim().to_string()),
            node,
        },
    ))
}

pub fn build_remove_virtual_io_node_edit(uuid: &str) -> ProjectEdit {
    ProjectEdit::RemoveTreeNode {
        node_uuid: uuid.to_string(),
    }
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

    #[test]
    fn multiplier_unit_upsert_and_instantiate_roundtrip() {
        let mut p = parse_loxone_xml(XML).unwrap();
        assert!(!is_multiplier_project(&p));

        let unit = MultiplierUnitEntry {
            uuid: String::new(),
            title: "Suite 201".to_string(),
            miniserver_serial: "504f94ff0201".to_string(),
            address: "192.168.10.201".to_string(),
            description: "2nd floor corner suite".to_string(),
            device_serial_overrides: vec![("u1".to_string(), "b0301234".to_string())],
        };
        let (unit_uuid, edit) = build_upsert_multiplier_unit_edit(&p, &unit);
        apply_edit(&mut p, &edit).unwrap();

        assert!(is_multiplier_project(&p));
        let units = collect_multiplier_units(&p);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].uuid, unit_uuid);
        assert_eq!(units[0].title, "Suite 201");
        assert_eq!(units[0].miniserver_serial, "504F94FF0201");
        assert_eq!(
            units[0].device_serial_overrides,
            vec![("u1".to_string(), "B0301234".to_string())]
        );

        let derived = instantiate_multiplier_unit_project(&p, &unit_uuid)
            .expect("should instantiate unit project");
        assert_eq!(derived.metadata.title, "Home — Suite 201");
        let u1_node = find_node_by_uuid(&derived, "u1").unwrap();
        assert_eq!(u1_node.attr("Serial"), Some("B0301234"));
    }

    #[test]
    fn new_project_creation_and_miniserver_model_conversion_roundtrip() {
        let spec = NewProjectSpec::default();
        let mut proj = create_new_project(&spec).expect("create new project");
        assert_eq!(proj.metadata.title, "New Smart Home");
        assert_eq!(detect_miniserver_model(&proj), MiniserverModel::Gen2);
        assert!(proj.places.len() >= 7);
        assert!(proj.pages.len() >= 6);

        // Convert Gen2 -> Compact -> Gen1
        let to_compact =
            build_convert_miniserver_model_edit(&proj, MiniserverModel::Compact).unwrap();
        apply_edit(&mut proj, &to_compact).unwrap();
        assert_eq!(detect_miniserver_model(&proj), MiniserverModel::Compact);
        assert!(find_first_by_type(&proj, "LoxAIR").is_some());

        let to_gen1 = build_convert_miniserver_model_edit(&proj, MiniserverModel::Gen1).unwrap();
        apply_edit(&mut proj, &to_gen1).unwrap();
        assert_eq!(detect_miniserver_model(&proj), MiniserverModel::Gen1);
        assert!(find_first_by_type(&proj, "AnalogOutputCaption").is_some());

        let xml = serialize_loxone_xml(&proj).unwrap();
        let reparsed = parse_loxone_xml(&xml).unwrap();
        assert_eq!(detect_miniserver_model(&reparsed), MiniserverModel::Gen1);
    }

    #[test]
    fn schematic_macro_instantiate_and_extract_roundtrip() {
        let mut proj = create_new_project(&NewProjectSpec::default()).unwrap();
        let page_uuid = proj.pages[0].uuid.clone();
        let macros = builtin_schematic_macros();
        assert!(macros.len() >= 3);

        let (created_uuids, edit) = build_instantiate_macro_edits(
            &proj,
            &page_uuid,
            &macros[0],
            128.0,
            128.0,
            Some("Bedroom"),
        )
        .unwrap();
        assert_eq!(created_uuids.len(), 3);
        apply_edit(&mut proj, &edit).unwrap();

        let page = proj.pages.iter().find(|p| p.uuid == page_uuid).unwrap();
        assert_eq!(page.blocks.len(), 3);

        let extracted = extract_macro_from_page(
            page,
            &created_uuids,
            "custom_bedroom",
            "Bedroom Macro",
            "Lighting",
            "Extracted from bedroom page",
        )
        .expect("extract macro");
        assert_eq!(extracted.blocks.len(), 3);
        assert_eq!(extracted.wires.len(), 2);
    }

    #[test]
    fn virtual_out_and_cmd_upsert_roundtrip() {
        let mut proj = create_new_project(&NewProjectSpec::default()).unwrap();
        assert!(collect_virtual_out_endpoints(&proj).is_empty());

        let (vo_uuid, vo_edit) = build_upsert_virtual_out_edit(
            &proj,
            &VirtualOutEndpoint {
                uuid: String::new(),
                i_name: "VQ1".to_string(),
                title: "InfluxDB Metrics".to_string(),
                address: "http://192.168.1.50:8086".to_string(),
                close_after_send: true,
                cmd_sep: ";".to_string(),
                room_uuid: None,
                commands: Vec::new(),
            },
        )
        .unwrap();
        apply_edit(&mut proj, &vo_edit).unwrap();

        let (cmd_uuid, cmd_edit) = build_upsert_virtual_out_cmd_edit(
            &proj,
            &VirtualOutCommandEntry {
                uuid: String::new(),
                parent_vo_uuid: vo_uuid.clone(),
                i_name: "VQC1".to_string(),
                title: "Post Room Temp".to_string(),
                analog: true,
                cmd_on: "/write?db=loxone".to_string(),
                cmd_on_post: "temp value=<v.1>".to_string(),
                cmd_on_method: "1".to_string(),
                cmd_off: String::new(),
                cmd_off_post: String::new(),
                repeat_sec: 30,
                room_uuid: None,
                category_uuid: None,
            },
        )
        .unwrap();
        apply_edit(&mut proj, &cmd_edit).unwrap();

        let endpoints = collect_virtual_out_endpoints(&proj);
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].uuid, vo_uuid);
        assert_eq!(endpoints[0].commands.len(), 1);
        assert_eq!(endpoints[0].commands[0].uuid, cmd_uuid);
        assert_eq!(endpoints[0].commands[0].cmd_on_post, "temp value=<v.1>");
    }
}
