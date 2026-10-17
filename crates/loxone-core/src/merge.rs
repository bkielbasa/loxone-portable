use std::collections::{HashMap, HashSet};

use crate::model::{
    parse_rgb_color, Category, LoxoneProject, Page, PageBlock, Place, TreeNode,
};
use crate::parser::LoxoneError;
use crate::writer::serialize_loxone_xml;
use crate::xref::XrefIndex;

/// Summary of differences detected when the Miniserver holds a newer project version than local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerDiffSummary {
    pub local_save_level: Option<u32>,
    pub remote_save_level: Option<u32>,
    pub local_c_date: String,
    pub remote_c_date: String,
    pub remote_filename: String,
    pub reason: String,
    pub remote_modified_or_added_blocks: usize,
    pub remote_removed_blocks: usize,
    pub remote_modified_or_added_tree_nodes: usize,
    pub remote_modified_or_added_pages: usize,
    pub local_unmerged_edits: usize,
    pub change_highlights: Vec<String>,
}

/// Payload returned when an upload is blocked because the Miniserver has a newer project version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerVersionConflict {
    pub remote_filename: String,
    pub remote_xml: String,
    pub diff: ServerDiffSummary,
}

/// Summary of a server-priority merge (`merge_remote_project_server_priority`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSummary {
    pub new_save_level: Option<u32>,
    pub new_c_date: String,
    pub server_items_applied: usize,
    pub local_additions_preserved: usize,
    pub local_edits_preserved: usize,
    pub conflicts_resolved_for_server: usize,
    pub details: Vec<String>,
}

/// Extracts a `YYYY-MM-DD HH:MM:SS` timestamp from a Loxone `/prog/` archive filename
/// such as `sps_222_20261015073732.zip` or `sps_222_20261015073732.LoxCC`.
pub fn extract_prog_archive_timestamp(filename: &str) -> Option<String> {
    let stem = filename
        .rsplit('/')
        .next()
        .unwrap_or(filename)
        .split('.')
        .next()
        .unwrap_or(filename);
    let digits = stem.rsplit('_').next()?;
    if digits.len() == 14 && digits.chars().all(|c| c.is_ascii_digit()) {
        Some(format!(
            "{}-{}-{} {}:{}:{}",
            &digits[0..4],
            &digits[4..6],
            &digits[6..8],
            &digits[8..10],
            &digits[10..12],
            &digits[12..14]
        ))
    } else {
        None
    }
}

fn collect_blocks_map(project: &LoxoneProject) -> HashMap<String, (&Page, &PageBlock)> {
    let mut map = HashMap::new();
    for page in &project.pages {
        for block in &page.blocks {
            if !block.uuid.is_empty() {
                map.insert(block.uuid.clone(), (page, block));
            }
        }
    }
    map
}

fn collect_tree_nodes_map<'a>(
    nodes: &'a [TreeNode],
    out: &mut HashMap<String, &'a TreeNode>,
) {
    for node in nodes {
        if !node.uuid.is_empty() {
            out.insert(node.uuid.clone(), node);
        }
        collect_tree_nodes_map(&node.children, out);
    }
}

fn all_tree_nodes_map(project: &LoxoneProject) -> HashMap<String, &TreeNode> {
    let mut map = HashMap::new();
    collect_tree_nodes_map(&project.periphery_tree, &mut map);
    collect_tree_nodes_map(&project.system_tree, &mut map);
    map
}

fn tree_node_shallow_eq(a: &TreeNode, b: &TreeNode) -> bool {
    a.control_type == b.control_type
        && a.title == b.title
        && a.room_uuid == b.room_uuid
        && a.category_uuid == b.category_uuid
        && a.pins == b.pins
        && a.raw_attributes == b.raw_attributes
        && a.extra_children == b.extra_children
}

/// Computes structural differences between a baseline project (`comparison_base`, which is
/// `base.unwrap_or(local)`) and `remote`, plus local unmerged edits relative to `base`.
fn compute_diff_stats(
    local: &LoxoneProject,
    remote: &LoxoneProject,
    base: Option<&LoxoneProject>,
) -> (usize, usize, usize, usize, usize, Vec<String>) {
    let ref_proj = base.unwrap_or(local);
    let ref_blocks = collect_blocks_map(ref_proj);
    let remote_blocks = collect_blocks_map(remote);
    let local_blocks = collect_blocks_map(local);

    let mut remote_mod_or_add_blocks = 0usize;
    let mut remote_removed_blocks = 0usize;
    let mut highlights = Vec::new();

    for (uuid, (r_page, r_blk)) in &remote_blocks {
        match ref_blocks.get(uuid) {
            None => {
                remote_mod_or_add_blocks += 1;
                if highlights.len() < 10 {
                    highlights.push(format!(
                        "Server added block '{}' ({}) on page '{}'",
                        r_blk.title, r_blk.block_type, r_page.title
                    ));
                }
            }
            Some((_, b_blk)) if b_blk != r_blk => {
                remote_mod_or_add_blocks += 1;
                if highlights.len() < 10 {
                    highlights.push(format!(
                        "Server modified block '{}' ({}) on page '{}'",
                        r_blk.title, r_blk.block_type, r_page.title
                    ));
                }
            }
            _ => {}
        }
    }

    for (uuid, (b_page, b_blk)) in &ref_blocks {
        if !remote_blocks.contains_key(uuid) {
            remote_removed_blocks += 1;
            if highlights.len() < 10 {
                highlights.push(format!(
                    "Server removed block '{}' ({}) from page '{}'",
                    b_blk.title, b_blk.block_type, b_page.title
                ));
            }
        }
    }

    let ref_nodes = all_tree_nodes_map(ref_proj);
    let remote_nodes = all_tree_nodes_map(remote);
    let mut remote_mod_or_add_nodes = 0usize;
    for (uuid, r_node) in &remote_nodes {
        match ref_nodes.get(uuid) {
            None => {
                remote_mod_or_add_nodes += 1;
                if highlights.len() < 10 {
                    highlights.push(format!(
                        "Server added tree item '{}' ({})",
                        r_node.title, r_node.control_type
                    ));
                }
            }
            Some(b_node) if !tree_node_shallow_eq(b_node, r_node) => {
                remote_mod_or_add_nodes += 1;
                if highlights.len() < 10 {
                    highlights.push(format!(
                        "Server modified tree item '{}' ({})",
                        r_node.title, r_node.control_type
                    ));
                }
            }
            _ => {}
        }
    }

    let ref_pages: HashMap<&str, &Page> =
        ref_proj.pages.iter().map(|p| (p.uuid.as_str(), p)).collect();
    let mut remote_mod_or_add_pages = 0usize;
    for r_page in &remote.pages {
        match ref_pages.get(r_page.uuid.as_str()) {
            None => {
                remote_mod_or_add_pages += 1;
                if highlights.len() < 10 {
                    highlights.push(format!("Server added page '{}'", r_page.title));
                }
            }
            Some(b_page)
                if b_page.title != r_page.title
                    || b_page.room_uuid != r_page.room_uuid
                    || b_page.category_uuid != r_page.category_uuid =>
            {
                remote_mod_or_add_pages += 1;
                if highlights.len() < 10 {
                    highlights.push(format!("Server updated page '{}'", r_page.title));
                }
            }
            _ => {}
        }
    }

    let mut local_unmerged_edits = 0usize;
    if let Some(base_proj) = base {
        let base_blocks = collect_blocks_map(base_proj);
        for (uuid, (_, l_blk)) in &local_blocks {
            match base_blocks.get(uuid) {
                None => local_unmerged_edits += 1,
                Some((_, b_blk)) if b_blk != l_blk => local_unmerged_edits += 1,
                _ => {}
            }
        }
        let base_nodes = all_tree_nodes_map(base_proj);
        let local_nodes = all_tree_nodes_map(local);
        for (uuid, l_node) in &local_nodes {
            match base_nodes.get(uuid) {
                None => local_unmerged_edits += 1,
                Some(b_node) if !tree_node_shallow_eq(b_node, l_node) => {
                    local_unmerged_edits += 1;
                }
                _ => {}
            }
        }
    } else {
        for (uuid, (_, l_blk)) in &local_blocks {
            match remote_blocks.get(uuid) {
                None => local_unmerged_edits += 1,
                Some((_, r_blk)) if r_blk != l_blk => local_unmerged_edits += 1,
                _ => {}
            }
        }
    }

    (
        remote_mod_or_add_blocks,
        remote_removed_blocks,
        remote_mod_or_add_nodes,
        remote_mod_or_add_pages,
        local_unmerged_edits,
        highlights,
    )
}
/// Checks whether the Miniserver (`remote`) holds a newer project version or unmerged changes
/// compared to `local` (and optional `base` / `last_synced_remote_xml`).
///
/// Returns `Some(ServerDiffSummary)` if the upload should be blocked by the safeguard so the user
/// can pull and merge the server changes first.
pub fn check_server_newer_version(
    local: &LoxoneProject,
    remote: &LoxoneProject,
    remote_filename: &str,
    base: Option<&LoxoneProject>,
    last_synced_remote_xml: Option<&str>,
) -> Option<ServerDiffSummary> {
    let (
        remote_modified_or_added_blocks,
        remote_removed_blocks,
        remote_modified_or_added_tree_nodes,
        remote_modified_or_added_pages,
        local_unmerged_edits,
        mut change_highlights,
    ) = compute_diff_stats(local, remote, base);

    let remote_has_structural_changes = remote_modified_or_added_blocks > 0
        || remote_removed_blocks > 0
        || remote_modified_or_added_tree_nodes > 0
        || remote_modified_or_added_pages > 0
        || base.unwrap_or(local).places != remote.places
        || base.unwrap_or(local).categories != remote.categories;

    let mut reason: Option<String> = None;

    // 1. Check Save Level (SL) on Document root against local or base
    if let (Some(r_sl), Some(l_sl)) = (remote.metadata.save_level, local.metadata.save_level) {
        if r_sl > l_sl {
            reason = Some(format!(
                "Miniserver Save Level (SL={}) is newer than local project (SL={})",
                r_sl, l_sl
            ));
        }
    }
    if reason.is_none() {
        if let (Some(r_sl), Some(b_sl)) = (
            remote.metadata.save_level,
            base.and_then(|b| b.metadata.save_level),
        ) {
            if r_sl > b_sl {
                reason = Some(format!(
                    "Miniserver Save Level (SL={}) advanced since your baseline (SL={})",
                    r_sl, b_sl
                ));
            }
        }
    }

    // 2. Check DateS (Loxone internal seconds timestamp) if SL did not explicitly decrease
    let sl_explicitly_older_on_remote = match (remote.metadata.save_level, local.metadata.save_level) {
        (Some(r_sl), Some(l_sl)) => r_sl < l_sl,
        _ => false,
    };

    if reason.is_none() && !sl_explicitly_older_on_remote {
        if let (Some(r_ds), Some(l_ds)) = (remote.metadata.date_s, local.metadata.date_s) {
            if r_ds > l_ds {
                reason = Some(format!(
                    "Miniserver timestamp (DateS={}) is newer than local project (DateS={})",
                    r_ds, l_ds
                ));
            }
        }
    }

    // 3. Check CDate ("YYYY-MM-DD HH:MM:SS") modification timestamp
    if reason.is_none()
        && !sl_explicitly_older_on_remote
        && !remote.metadata.c_date.is_empty()
        && !local.metadata.c_date.is_empty()
        && remote.metadata.c_date > local.metadata.c_date
    {
        reason = Some(format!(
            "Miniserver modification date ({}) is newer than local project ({})",
            remote.metadata.c_date, local.metadata.c_date
        ));
    }

    // 4. Check /prog/sps_<ver>_<YYYYMMDDHHMMSS>.zip archive timestamp when structural differences exist
    if reason.is_none() && !sl_explicitly_older_on_remote && remote_has_structural_changes {
        if let Some(archive_dt) = extract_prog_archive_timestamp(remote_filename) {
            if !local.metadata.c_date.is_empty() && archive_dt > local.metadata.c_date {
                reason = Some(format!(
                    "Miniserver archive '{}' ({}) is newer than local project ({})",
                    remote_filename, archive_dt, local.metadata.c_date
                ));
            }
        }
    }

    // 5. Check if the Miniserver XML changed since `last_synced_remote_xml` or `base`
    if reason.is_none() {
        if let Some(synced_xml) = last_synced_remote_xml {
            if remote.raw_xml.as_deref() != Some(synced_xml) && remote_has_structural_changes {
                reason = Some(
                    "Miniserver project was modified on the server since your last sync".to_string(),
                );
            }
        } else if base.is_some() && remote_has_structural_changes && !sl_explicitly_older_on_remote {
            reason = Some(
                "Miniserver has unmerged changes compared to your project baseline".to_string(),
            );
        }
    }

    let reason = reason?;

    if change_highlights.is_empty() {
        change_highlights.push(format!(
            "Server project metadata updated (SL={:?}, CDate='{}')",
            remote.metadata.save_level, remote.metadata.c_date
        ));
    }

    Some(ServerDiffSummary {
        local_save_level: local.metadata.save_level,
        remote_save_level: remote.metadata.save_level,
        local_c_date: local.metadata.c_date.clone(),
        remote_c_date: remote.metadata.c_date.clone(),
        remote_filename: remote_filename.to_string(),
        reason,
        remote_modified_or_added_blocks,
        remote_removed_blocks,
        remote_modified_or_added_tree_nodes,
        remote_modified_or_added_pages,
        local_unmerged_edits,
        change_highlights,
    })
}

fn sync_block_cached_fields(block: &mut PageBlock) {
    let get_attr = |attrs: &[(String, String)], key: &str| -> Option<String> {
        attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    if let Some(title) = get_attr(&block.raw_attributes, "Title") {
        block.title = title;
    }
    if let Some(px) = get_attr(&block.raw_attributes, "Px").and_then(|v| v.parse::<f32>().ok()) {
        let dx = px - block.px;
        block.px = px;
        block.px2 += dx;
    }
    if let Some(py) = get_attr(&block.raw_attributes, "Py").and_then(|v| v.parse::<f32>().ok()) {
        let dy = py - block.py;
        block.py = py;
        block.py2 += dy;
    }
    block.color_rgb = get_attr(&block.raw_attributes, "Cl").and_then(|v| parse_rgb_color(&v));
    if let Some(r) = get_attr(&block.raw_attributes, "Ref") {
        block.ref_uuid = Some(r);
    }
    if let Some(io) = block.extra_children.iter().find(|c| c.tag == "IoData") {
        block.room_uuid = io.attr("Pr").map(|s| s.to_string());
        block.category_uuid = io.attr("Cr").map(|s| s.to_string());
    }
}

fn merge_page_block_server_priority(
    remote_block: &PageBlock,
    local_block: &PageBlock,
    base_block: Option<&PageBlock>,
    local_edits_preserved: &mut usize,
    conflicts_resolved_for_server: &mut usize,
    details: &mut Vec<String>,
) -> PageBlock {
    let mut merged = remote_block.clone();
    if remote_block == local_block {
        return merged;
    }

    let Some(base_blk) = base_block else {
        // Without a common 3-way base, what is on the server takes strictly higher priority
        // for any shared block that differs between local and remote.
        *conflicts_resolved_for_server += 1;
        if details.len() < 12 {
            details.push(format!(
                "Server priority kept server version of block '{}' ({})",
                remote_block.title, remote_block.block_type
            ));
        }
        return merged;
    };

    // 3-way merge with strict server priority on any conflict:
    // 1. Merge raw_attributes
    let remote_attrs: HashMap<&str, &str> = remote_block
        .raw_attributes
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let local_attrs: HashMap<&str, &str> = local_block
        .raw_attributes
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let base_attrs: HashMap<&str, &str> = base_blk
        .raw_attributes
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let mut all_keys: Vec<&str> = Vec::new();
    for (k, _) in &remote_block.raw_attributes {
        if !all_keys.contains(&k.as_str()) {
            all_keys.push(k.as_str());
        }
    }
    for (k, _) in &local_block.raw_attributes {
        if !all_keys.contains(&k.as_str()) {
            all_keys.push(k.as_str());
        }
    }

    for key in all_keys {
        let r_val = remote_attrs.get(key).copied();
        let l_val = local_attrs.get(key).copied();
        let b_val = base_attrs.get(key).copied();

        if r_val == l_val {
            continue;
        }
        if r_val == b_val && l_val != b_val {
            // Unconflicted local edit: server didn't touch this attribute
            match l_val {
                Some(v) => {
                    if let Some((_, slot)) =
                        merged.raw_attributes.iter_mut().find(|(k, _)| k == key)
                    {
                        *slot = v.to_string();
                    } else {
                        merged.raw_attributes.push((key.to_string(), v.to_string()));
                    }
                }
                None => {
                    merged.raw_attributes.retain(|(k, _)| k != key);
                }
            }
            *local_edits_preserved += 1;
            if details.len() < 12 {
                details.push(format!(
                    "Preserved local edit on '{}' attribute '{}'",
                    merged.title, key
                ));
            }
        } else {
            // Server modified this attribute (and local either didn't or conflicted) -> Server wins!
            if l_val != b_val {
                *conflicts_resolved_for_server += 1;
                if details.len() < 12 {
                    details.push(format!(
                        "Server priority won on '{}' attribute '{}' (kept '{}')",
                        remote_block.title,
                        key,
                        r_val.unwrap_or("<removed>")
                    ));
                }
            }
        }
    }

    // 2. Merge text_content (for Note / Text blocks)
    if remote_block.text_content != local_block.text_content {
        if remote_block.text_content == base_blk.text_content {
            merged.text_content = local_block.text_content.clone();
            *local_edits_preserved += 1;
        } else if local_block.text_content != base_blk.text_content {
            *conflicts_resolved_for_server += 1;
        }
    }

    // 3. Merge extra_children (IoData, LoggerMailer, HCL, Display, etc.)
    if remote_block.extra_children != local_block.extra_children {
        if remote_block.extra_children == base_blk.extra_children {
            merged.extra_children = local_block.extra_children.clone();
            *local_edits_preserved += 1;
        } else if local_block.extra_children != base_blk.extra_children {
            *conflicts_resolved_for_server += 1;
        }
    }

    // 4. Merge ConnectorPins (default values & incoming wire links)
    for local_pin in &local_block.pins {
        let base_pin = base_blk.pins.iter().find(|p| p.key == local_pin.key);
        if let Some(merged_pin) = merged.pins.iter_mut().find(|p| p.key == local_pin.key) {
            if merged_pin.default_value != local_pin.default_value {
                let b_def = base_pin.and_then(|p| p.default_value.as_deref());
                if merged_pin.default_value.as_deref() == b_def {
                    merged_pin.default_value = local_pin.default_value.clone();
                    *local_edits_preserved += 1;
                } else if local_pin.default_value.as_deref() != b_def {
                    *conflicts_resolved_for_server += 1;
                    if details.len() < 12 {
                        details.push(format!(
                            "Server priority won on '{}' pin '{}' parameter",
                            remote_block.title, local_pin.key
                        ));
                    }
                }
            }
            if merged_pin.incoming_links != local_pin.incoming_links {
                let b_links = base_pin.map(|p| p.incoming_links.as_slice()).unwrap_or(&[]);
                if merged_pin.incoming_links.as_slice() == b_links {
                    merged_pin.incoming_links = local_pin.incoming_links.clone();
                    *local_edits_preserved += 1;
                } else if local_pin.incoming_links.as_slice() != b_links {
                    *conflicts_resolved_for_server += 1;
                }
            }
        } else if base_pin.is_none() {
            // Pin was newly added locally
            merged.pins.push(local_pin.clone());
            *local_edits_preserved += 1;
        }
    }

    sync_block_cached_fields(&mut merged);
    merged
}

fn merge_tree_nodes_recursive(
    merged_nodes: &mut Vec<TreeNode>,
    local_nodes: &[TreeNode],
    base_nodes: Option<&[TreeNode]>,
    remote_all_uuids: &HashSet<String>,
    base_all_uuids: &HashSet<String>,
    local_additions_preserved: &mut usize,
    local_edits_preserved: &mut usize,
    conflicts_resolved_for_server: &mut usize,
    details: &mut Vec<String>,
) {
    let local_map: HashMap<&str, &TreeNode> =
        local_nodes.iter().map(|n| (n.uuid.as_str(), n)).collect();
    let base_map: HashMap<&str, &TreeNode> = base_nodes
        .unwrap_or(&[])
        .iter()
        .map(|n| (n.uuid.as_str(), n))
        .collect();

    for m_node in merged_nodes.iter_mut() {
        if let Some(l_node) = local_map.get(m_node.uuid.as_str()) {
            let b_node = base_map.get(m_node.uuid.as_str()).copied();
            if !tree_node_shallow_eq(m_node, l_node) {
                if let Some(b) = b_node {
                    if tree_node_shallow_eq(m_node, b) {
                        // Server did not touch this tree node, preserve local edit
                        m_node.title = l_node.title.clone();
                        m_node.room_uuid = l_node.room_uuid.clone();
                        m_node.category_uuid = l_node.category_uuid.clone();
                        m_node.pins = l_node.pins.clone();
                        m_node.raw_attributes = l_node.raw_attributes.clone();
                        m_node.extra_children = l_node.extra_children.clone();
                        *local_edits_preserved += 1;
                        if details.len() < 12 {
                            details.push(format!(
                                "Preserved local edit on tree item '{}'",
                                m_node.title
                            ));
                        }
                    } else {
                        // Server changed this tree node -> Server wins!
                        *conflicts_resolved_for_server += 1;
                        if details.len() < 12 {
                            details.push(format!(
                                "Server priority kept server version of tree item '{}'",
                                m_node.title
                            ));
                        }
                    }
                } else {
                    *conflicts_resolved_for_server += 1;
                }
            }
            merge_tree_nodes_recursive(
                &mut m_node.children,
                &l_node.children,
                b_node.map(|b| b.children.as_slice()),
                remote_all_uuids,
                base_all_uuids,
                local_additions_preserved,
                local_edits_preserved,
                conflicts_resolved_for_server,
                details,
            );
        }
    }

    // Preserve local-only tree nodes added at this level (unless deleted on server relative to base)
    for l_node in local_nodes {
        if !l_node.uuid.is_empty() && !remote_all_uuids.contains(&l_node.uuid) {
            if !base_all_uuids.contains(&l_node.uuid) {
                merged_nodes.push(l_node.clone());
                *local_additions_preserved += 1;
                if details.len() < 12 {
                    details.push(format!(
                        "Preserved local addition of tree item '{}' ({})",
                        l_node.title, l_node.control_type
                    ));
                }
            } else {
                *conflicts_resolved_for_server += 1;
            }
        }
    }
}

/// Merges the Miniserver project (`remote`) into `local` where **what is on the server takes
/// higher priority** on any conflict, while preserving non-conflicting local additions and
/// unconflicted local edits (when `base` is available).
pub fn merge_remote_project_server_priority(
    local: &mut LoxoneProject,
    remote: &LoxoneProject,
    base: Option<&LoxoneProject>,
) -> Result<MergeSummary, LoxoneError> {
    let (
        remote_mod_blocks,
        remote_rem_blocks,
        remote_mod_nodes,
        remote_mod_pages,
        _,
        _,
    ) = compute_diff_stats(local, remote, base);
    let server_items_applied =
        remote_mod_blocks + remote_rem_blocks + remote_mod_nodes + remote_mod_pages;

    let mut merged = remote.clone();
    let mut local_additions_preserved = 0usize;
    let mut local_edits_preserved = 0usize;
    let mut conflicts_resolved_for_server = 0usize;
    let mut details = Vec::new();

    // 1. Merge Places (Rooms) with server priority
    for (uuid, l_place) in &local.places {
        match merged.places.get(uuid) {
            None => {
                let was_in_base = base.is_some_and(|b| b.places.contains_key(uuid));
                if !was_in_base {
                    merged.places.insert(uuid.clone(), l_place.clone());
                    local_additions_preserved += 1;
                    if details.len() < 12 {
                        details.push(format!("Preserved local room '{}'", l_place.title));
                    }
                } else {
                    conflicts_resolved_for_server += 1;
                }
            }
            Some(r_place) if r_place != l_place => {
                let b_place: Option<&Place> = base.and_then(|b| b.places.get(uuid));
                if b_place == Some(r_place) {
                    merged.places.insert(uuid.clone(), l_place.clone());
                    local_edits_preserved += 1;
                } else {
                    conflicts_resolved_for_server += 1;
                }
            }
            _ => {}
        }
    }

    // 2. Merge Categories with server priority
    for (uuid, l_cat) in &local.categories {
        match merged.categories.get(uuid) {
            None => {
                let was_in_base = base.is_some_and(|b| b.categories.contains_key(uuid));
                if !was_in_base {
                    merged.categories.insert(uuid.clone(), l_cat.clone());
                    local_additions_preserved += 1;
                    if details.len() < 12 {
                        details.push(format!("Preserved local category '{}'", l_cat.title));
                    }
                } else {
                    conflicts_resolved_for_server += 1;
                }
            }
            Some(r_cat) if r_cat != l_cat => {
                let b_cat: Option<&Category> = base.and_then(|b| b.categories.get(uuid));
                if b_cat == Some(r_cat) {
                    merged.categories.insert(uuid.clone(), l_cat.clone());
                    local_edits_preserved += 1;
                } else {
                    conflicts_resolved_for_server += 1;
                }
            }
            _ => {}
        }
    }

    // 3. Merge Periphery & System Trees with server priority
    let remote_tree_uuids: HashSet<String> = all_tree_nodes_map(remote).into_keys().collect();
    let base_tree_uuids: HashSet<String> = base
        .map(|b| all_tree_nodes_map(b).into_keys().collect())
        .unwrap_or_default();

    merge_tree_nodes_recursive(
        &mut merged.periphery_tree,
        &local.periphery_tree,
        base.map(|b| b.periphery_tree.as_slice()),
        &remote_tree_uuids,
        &base_tree_uuids,
        &mut local_additions_preserved,
        &mut local_edits_preserved,
        &mut conflicts_resolved_for_server,
        &mut details,
    );
    merge_tree_nodes_recursive(
        &mut merged.system_tree,
        &local.system_tree,
        base.map(|b| b.system_tree.as_slice()),
        &remote_tree_uuids,
        &base_tree_uuids,
        &mut local_additions_preserved,
        &mut local_edits_preserved,
        &mut conflicts_resolved_for_server,
        &mut details,
    );

    // 4. Merge Pages & PageBlocks with server priority
    let remote_block_uuids: HashSet<String> = collect_blocks_map(remote).into_keys().collect();
    let base_blocks_map = base.map(collect_blocks_map).unwrap_or_default();
    let base_block_uuids: HashSet<String> = base_blocks_map.keys().cloned().collect();
    let local_blocks_map = collect_blocks_map(local);

    let local_pages_map: HashMap<&str, &Page> =
        local.pages.iter().map(|p| (p.uuid.as_str(), p)).collect();
    let base_pages_map: HashMap<&str, &Page> = base
        .map(|b| b.pages.iter().map(|p| (p.uuid.as_str(), p)).collect())
        .unwrap_or_default();
    let remote_page_uuids: HashSet<String> =
        remote.pages.iter().map(|p| p.uuid.clone()).collect();

    for m_page in &mut merged.pages {
        if let Some(l_page) = local_pages_map.get(m_page.uuid.as_str()) {
            let b_page = base_pages_map.get(m_page.uuid.as_str()).copied();
            // Merge page header (title, room_uuid, category_uuid)
            let page_header_differs = m_page.title != l_page.title
                || m_page.room_uuid != l_page.room_uuid
                || m_page.category_uuid != l_page.category_uuid;
            if page_header_differs {
                if let Some(bp) = b_page {
                    let remote_unchanged = m_page.title == bp.title
                        && m_page.room_uuid == bp.room_uuid
                        && m_page.category_uuid == bp.category_uuid;
                    if remote_unchanged {
                        m_page.title = l_page.title.clone();
                        m_page.room_uuid = l_page.room_uuid.clone();
                        m_page.category_uuid = l_page.category_uuid.clone();
                        local_edits_preserved += 1;
                    } else {
                        conflicts_resolved_for_server += 1;
                    }
                } else {
                    conflicts_resolved_for_server += 1;
                }
            }

            // Merge blocks that exist on both server and local
            for m_blk in &mut m_page.blocks {
                if let Some((_, l_blk)) = local_blocks_map.get(&m_blk.uuid) {
                    let b_blk = base_blocks_map.get(&m_blk.uuid).map(|(_, b)| *b);
                    *m_blk = merge_page_block_server_priority(
                        m_blk,
                        l_blk,
                        b_blk,
                        &mut local_edits_preserved,
                        &mut conflicts_resolved_for_server,
                        &mut details,
                    );
                }
            }

            // Preserve local-only blocks on this page (unless deleted on server relative to base)
            for l_blk in &l_page.blocks {
                if !l_blk.uuid.is_empty() && !remote_block_uuids.contains(&l_blk.uuid) {
                    if !base_block_uuids.contains(&l_blk.uuid) {
                        m_page.blocks.push(l_blk.clone());
                        local_additions_preserved += 1;
                        if details.len() < 12 {
                            details.push(format!(
                                "Preserved local block '{}' ({}) on page '{}'",
                                l_blk.title, l_blk.block_type, m_page.title
                            ));
                        }
                    } else {
                        conflicts_resolved_for_server += 1;
                    }
                }
            }
        }
    }

    // Preserve local-only pages (unless deleted on server relative to base)
    for l_page in &local.pages {
        if !l_page.uuid.is_empty() && !remote_page_uuids.contains(&l_page.uuid) {
            if !base_pages_map.contains_key(l_page.uuid.as_str()) {
                merged.pages.push(l_page.clone());
                local_additions_preserved += 1;
                if details.len() < 12 {
                    details.push(format!("Preserved local page '{}'", l_page.title));
                }
            } else {
                conflicts_resolved_for_server += 1;
            }
        }
    }

    merged.xref = XrefIndex::build(&merged);
    if merged.raw_xml.is_some() {
        let updated_xml = serialize_loxone_xml(&merged)?;
        merged.raw_xml = Some(updated_xml);
    }

    let summary = MergeSummary {
        new_save_level: merged.metadata.save_level,
        new_c_date: merged.metadata.c_date.clone(),
        server_items_applied,
        local_additions_preserved,
        local_edits_preserved,
        conflicts_resolved_for_server,
        details,
    };

    *local = merged;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::new_page_block;
    use crate::parser::parse_loxone_xml;
    use crate::writer::{apply_edit, ProjectEdit};

    const BASE_XML: &str = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\
<ControlList Version=\"1\">\r\n\
\t<C Type=\"Document\" Title=\"Home\" U=\"doc-1\" ConfigVersion=\"17020828\" CDate=\"2026-10-15 07:37:30\" DateS=\"559780177\" SL=\"271\">\r\n\
\t\t<C Type=\"Program\" Title=\"Program\">\r\n\
\t\t\t<C Type=\"Page\" Title=\"Salon\" U=\"page-1\">\r\n\
\t\t\t\t<C Type=\"Switch\" Title=\"Main Light\" U=\"blk-1\" Px=\"100\" Py=\"200\">\r\n\
\t\t\t\t\t<Co K=\"Delay\" U=\"pin-delay\" Def=\"60\"/>\r\n\
\t\t\t\t</C>\r\n\
\t\t\t</C>\r\n\
\t\t</C>\r\n\
\t</C>\r\n\
</ControlList>\r\n";

    #[test]
    fn test_extract_prog_archive_timestamp() {
        assert_eq!(
            extract_prog_archive_timestamp("sps_222_20261015073732.zip"),
            Some("2026-10-15 07:37:32".to_string())
        );
        assert_eq!(
            extract_prog_archive_timestamp("/prog/sps_225_20261017194500.LoxCC"),
            Some("2026-10-17 19:45:00".to_string())
        );
        assert_eq!(extract_prog_archive_timestamp("sps0.zip"), None);
    }

    #[test]
    fn test_check_server_newer_version_detects_higher_sl_and_cdate() {
        let local = parse_loxone_xml(BASE_XML).unwrap();
        let mut remote = parse_loxone_xml(BASE_XML).unwrap();
        remote.metadata.save_level = Some(274);
        remote.metadata.c_date = "2026-10-17 12:00:00".to_string();
        apply_edit(
            &mut remote,
            &ProjectEdit::SetControlAttr {
                control_uuid: "blk-1".to_string(),
                attr_name: "Title".to_string(),
                value: Some("Server Renamed Light".to_string()),
            },
        )
        .unwrap();

        let diff = check_server_newer_version(
            &local,
            &remote,
            "sps_222_20261017120000.zip",
            Some(&local),
            None,
        )
        .expect("expected newer server version detected");

        assert_eq!(diff.local_save_level, Some(271));
        assert_eq!(diff.remote_save_level, Some(274));
        assert_eq!(diff.remote_modified_or_added_blocks, 1);
        assert!(diff.reason.contains("SL=274"));
    }

    #[test]
    fn test_merge_remote_project_server_priority_wins_on_conflict_and_keeps_local_additions() {
        let base = parse_loxone_xml(BASE_XML).unwrap();
        let mut local = base.clone();
        let mut remote = base.clone();

        // 1. Conflicting edit on blk-1 Title: local sets "Local Light", server sets "Server Light"
        apply_edit(
            &mut local,
            &ProjectEdit::SetControlAttr {
                control_uuid: "blk-1".to_string(),
                attr_name: "Title".to_string(),
                value: Some("Local Light".to_string()),
            },
        )
        .unwrap();
        apply_edit(
            &mut remote,
            &ProjectEdit::SetControlAttr {
                control_uuid: "blk-1".to_string(),
                attr_name: "Title".to_string(),
                value: Some("Server Light".to_string()),
            },
        )
        .unwrap();

        // 2. Unconflicted local edit on blk-1 pin Delay: local sets 120, server leaves at 60
        apply_edit(
            &mut local,
            &ProjectEdit::SetPinDefault {
                control_uuid: "blk-1".to_string(),
                pin_key: "Delay".to_string(),
                default_value: Some("120".to_string()),
            },
        )
        .unwrap();

        // 3. Local-only new block added on page-1
        let mut new_blk = new_page_block("Pushbutton", "Local Dimmer", 300.0, 200.0);
        new_blk.uuid = "blk-local-new".to_string();
        apply_edit(
            &mut local,
            &ProjectEdit::AddPageBlock {
                page_uuid: "page-1".to_string(),
                block: new_blk,
            },
        )
        .unwrap();

        // 4. Server advances SL and CDate
        remote.metadata.save_level = Some(275);
        remote.metadata.c_date = "2026-10-17 18:00:00".to_string();
        remote.raw_xml = Some(serialize_loxone_xml(&remote).unwrap());

        let summary =
            merge_remote_project_server_priority(&mut local, &remote, Some(&base)).unwrap();

        assert_eq!(summary.new_save_level, Some(275));
        assert_eq!(local.metadata.save_level, Some(275));
        assert_eq!(local.metadata.c_date, "2026-10-17 18:00:00");
        assert!(summary.conflicts_resolved_for_server >= 1);
        assert!(summary.local_additions_preserved >= 1);
        assert!(summary.local_edits_preserved >= 1);

        let page = &local.pages[0];
        let blk1 = page.blocks.iter().find(|b| b.uuid == "blk-1").unwrap();
        // Server priority won on Title conflict!
        assert_eq!(blk1.title, "Server Light");
        // Unconflicted local edit on pin Delay was preserved!
        let delay_pin = blk1.pins.iter().find(|p| p.key == "Delay").unwrap();
        assert_eq!(delay_pin.default_value.as_deref(), Some("120"));
        // Local-only new block was preserved!
        assert!(page.blocks.iter().any(|b| b.uuid == "blk-local-new"));

        // And after merging, checking against the same remote version returns None (no longer blocked!)
        assert!(
            check_server_newer_version(
                &local,
                &remote,
                "sps_222_20261017180000.zip",
                Some(&remote),
                remote.raw_xml.as_deref(),
            )
            .is_none()
        );
    }
}
