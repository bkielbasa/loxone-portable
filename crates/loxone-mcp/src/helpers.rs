use loxone_core::factory::{new_page, new_page_block};
use loxone_core::model::{
    ConnectorPin, LoxoneProject, Page, PageBlock, PinKind, RawNode, TreeNode, TreeSectionKind,
};
use loxone_core::parser::LoxoneError;
use loxone_core::writer::ProjectEdit;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn build_clean_project(title: &str) -> Result<LoxoneProject, LoxoneError> {
    let mut spec = loxone_core::NewProjectSpec::default();
    if !title.trim().is_empty() {
        spec.title = title.trim().to_string();
    }
    loxone_core::create_new_project(&spec)
}

pub fn pin_kind_str(k: PinKind) -> &'static str {
    match k {
        PinKind::Input => "Input",
        PinKind::Parameter => "Parameter",
        PinKind::Output => "Output",
    }
}

pub fn pin_to_json(pin: &ConnectorPin, project: &LoxoneProject) -> Value {
    let links: Vec<Value> = pin
        .incoming_links
        .iter()
        .map(|l| {
            let src_meta = project.xref.pins_by_uuid.get(&l.source_pin_uuid);
            json!({
                "source_pin_uuid": l.source_pin_uuid,
                "source_block_uuid": src_meta.map(|m| m.owner_uuid.as_str()).unwrap_or(""),
                "source_pin_key": src_meta.map(|m| m.pin_key.as_str()).unwrap_or(""),
                "flags": l.flags,
                "hidden": l.is_hidden()
            })
        })
        .collect();
    json!({
        "uuid": pin.uuid,
        "key": pin.key,
        "kind": pin_kind_str(pin.pin_kind),
        "default_value": pin.default_value,
        "inverted": pin.inverted,
        "incoming_links": links
    })
}

pub fn block_to_json(block: &PageBlock, page: &Page, project: &LoxoneProject) -> Value {
    let room_title = block
        .room_uuid
        .as_ref()
        .and_then(|u| project.places.get(u))
        .map(|r| r.title.as_str())
        .unwrap_or("");
    let cat_title = block
        .category_uuid
        .as_ref()
        .and_then(|u| project.categories.get(u))
        .map(|c| c.title.as_str())
        .unwrap_or("");
    let pins: Vec<Value> = block.pins.iter().map(|p| pin_to_json(p, project)).collect();
    let attrs: serde_json::Map<String, Value> = block
        .raw_attributes
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    json!({
        "uuid": block.uuid,
        "block_type": block.block_type,
        "title": block.title,
        "page_uuid": page.uuid,
        "page_title": page.title,
        "has_position": block.has_position,
        "px": block.px,
        "py": block.py,
        "px2": block.px2,
        "py2": block.py2,
        "room_uuid": block.room_uuid,
        "room_title": room_title,
        "category_uuid": block.category_uuid,
        "category_title": cat_title,
        "ref_uuid": block.ref_uuid,
        "text_content": block.text_content,
        "attributes": attrs,
        "pins": pins
    })
}

pub fn tree_node_to_json(node: &TreeNode, project: &LoxoneProject) -> Value {
    let section = match node.section {
        TreeSectionKind::Periphery => "Periphery",
        TreeSectionKind::System => "System",
    };
    let pins: Vec<Value> = node.pins.iter().map(|p| pin_to_json(p, project)).collect();
    let children: Vec<Value> = node
        .children
        .iter()
        .map(|c| tree_node_to_json(c, project))
        .collect();
    let attrs: serde_json::Map<String, Value> = node
        .raw_attributes
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    json!({
        "uuid": node.uuid,
        "control_type": node.control_type,
        "title": node.title,
        "section": section,
        "room_uuid": node.room_uuid,
        "category_uuid": node.category_uuid,
        "serial": node.attr("Serial"),
        "attributes": attrs,
        "pins": pins,
        "children": children
    })
}

pub fn find_tree_node<'a>(project: &'a LoxoneProject, uuid: &str) -> Option<&'a TreeNode> {
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
    walk(&project.periphery_tree, uuid).or_else(|| walk(&project.system_tree, uuid))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfigRoomPlan {
    pub room_uuid: String,
    pub room_title: String,
    pub selected: bool,
    pub has_lighting: bool,
    pub has_shading: bool,
    pub has_climate: bool,
    pub has_audio: bool,
    pub gen_lighting: bool,
    pub gen_shading: bool,
    pub gen_climate: bool,
    pub gen_audio: bool,
}

pub fn build_auto_config_plans(
    project: &LoxoneProject,
    filter_rooms: Option<&[String]>,
    gen_lighting_opt: Option<bool>,
    gen_shading_opt: Option<bool>,
    gen_climate_opt: Option<bool>,
    gen_audio_opt: Option<bool>,
) -> (Vec<AutoConfigRoomPlan>, Option<ProjectEdit>, usize) {
    let mut places: Vec<(&String, &loxone_core::model::Place)> = project.places.iter().collect();
    places.sort_by(|a, b| a.1.title.cmp(&b.1.title));

    let mut plans = Vec::new();
    for (room_uuid, place) in places {
        let mut has_lighting = false;
        let mut has_shading = false;
        let mut has_climate = false;
        let mut has_audio = false;

        for page in &project.pages {
            let page_matches_room = page.title.eq_ignore_ascii_case(&place.title);
            for block in &page.blocks {
                let in_room = block.room_uuid.as_deref() == Some(room_uuid.as_str())
                    || (block.room_uuid.is_none() && page_matches_room);
                if !in_room {
                    continue;
                }
                match block.block_type.as_str() {
                    "LightController2" | "LightControllerV2" | "LightController" => {
                        has_lighting = true;
                    }
                    "AutoJalousie" | "Jalousie" => {
                        has_shading = true;
                    }
                    "HeatIRoomController2" | "IRoomControllerV2" | "IRoomController" => {
                        has_climate = true;
                    }
                    "AudioZone" | "MusicPlayer" | "MusicServerZone" => {
                        has_audio = true;
                    }
                    _ => {}
                }
            }
        }

        let gen_lighting = gen_lighting_opt.unwrap_or(true) && !has_lighting;
        let gen_shading = gen_shading_opt.unwrap_or(true) && !has_shading;
        let gen_climate = gen_climate_opt.unwrap_or(true) && !has_climate;
        let gen_audio = gen_audio_opt.unwrap_or(false) && !has_audio;
        let any_gen = gen_lighting || gen_shading || gen_climate || gen_audio;
        let selected = match filter_rooms {
            Some(list) => list.iter().any(|u| u == room_uuid),
            None => any_gen,
        };

        plans.push(AutoConfigRoomPlan {
            room_uuid: room_uuid.clone(),
            room_title: place.title.clone(),
            selected,
            has_lighting,
            has_shading,
            has_climate,
            has_audio,
            gen_lighting,
            gen_shading,
            gen_climate,
            gen_audio,
        });
    }

    let find_category_uuid = |group_hint: &str| -> Option<String> {
        project
            .categories
            .values()
            .find(|c| {
                c.cat_group
                    .as_deref()
                    .is_some_and(|g| g.eq_ignore_ascii_case(group_hint))
                    || c.title.to_lowercase().contains(&group_hint.to_lowercase())
            })
            .map(|c| c.uuid.clone())
    };
    let cat_lights = find_category_uuid("lights").or_else(|| find_category_uuid("oświetlenie"));
    let cat_shading = find_category_uuid("shading").or_else(|| find_category_uuid("zacienienie"));
    let cat_climate = find_category_uuid("climate")
        .or_else(|| find_category_uuid("temperatura"))
        .or_else(|| find_category_uuid("klimat"));
    let cat_audio = find_category_uuid("multimedia").or_else(|| find_category_uuid("audio"));

    let mut edits = Vec::new();
    let mut generated_blocks = 0usize;

    for row in plans.iter().filter(|r| r.selected) {
        let mut specs: Vec<(&str, String, Option<String>)> = Vec::new();
        if row.gen_lighting {
            specs.push((
                "LightController2",
                format!("{} — Lighting", row.room_title),
                cat_lights.clone(),
            ));
        }
        if row.gen_shading {
            specs.push((
                "AutoJalousie",
                format!("{} — Shading", row.room_title),
                cat_shading.clone(),
            ));
        }
        if row.gen_climate {
            specs.push((
                "HeatIRoomController2",
                format!("{} — Room Climate", row.room_title),
                cat_climate.clone(),
            ));
        }
        if row.gen_audio {
            specs.push((
                "AudioZone",
                format!("{} — Audio", row.room_title),
                cat_audio.clone(),
            ));
        }
        if specs.is_empty() {
            continue;
        }

        let (page_uuid, mut next_py) = if let Some(existing_page) = project
            .pages
            .iter()
            .find(|p| p.title.eq_ignore_ascii_case(&row.room_title))
        {
            let max_py = existing_page
                .blocks
                .iter()
                .filter(|b| b.has_position)
                .map(|b| b.py2.max(b.py + 640.0))
                .fold(192.0_f32, f32::max);
            (existing_page.uuid.clone(), max_py + 256.0)
        } else {
            let new_pg = new_page(&row.room_title);
            let pu = new_pg.uuid.clone();
            edits.push(ProjectEdit::AddPage { page: new_pg });
            (pu, 256.0_f32)
        };

        for (block_type, title, cat_uuid) in specs {
            let mut blk = new_page_block(block_type, &title, 1920.0, next_py);
            blk.room_uuid = Some(row.room_uuid.clone());
            blk.category_uuid = cat_uuid.clone();
            let mut iodata_attrs = vec![("Pr".to_string(), row.room_uuid.clone())];
            if let Some(cu) = cat_uuid {
                iodata_attrs.push(("Cr".to_string(), cu));
            }
            blk.extra_children.push(RawNode {
                tag: "IoData".to_string(),
                attributes: iodata_attrs,
                text: None,
                children: Vec::new(),
            });
            next_py += 896.0;
            generated_blocks += 1;
            edits.push(ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block: blk,
            });
        }
    }

    let batch = if edits.is_empty() {
        None
    } else {
        Some(ProjectEdit::Batch(edits))
    };
    (plans, batch, generated_blocks)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindReplaceItem {
    pub uuid: String,
    pub object_type: String,
    pub location: String,
    pub field_name: String,
    pub before: String,
    pub after: String,
}

fn replace_substring(
    src: &str,
    needle: &str,
    replacement: &str,
    case_sensitive: bool,
) -> Option<String> {
    if needle.is_empty() {
        return None;
    }
    if case_sensitive {
        if src.contains(needle) {
            Some(src.replace(needle, replacement))
        } else {
            None
        }
    } else {
        let src_lc = src.to_lowercase();
        let needle_lc = needle.to_lowercase();
        if !src_lc.contains(&needle_lc) {
            return None;
        }
        let mut out = String::with_capacity(src.len());
        let mut last_idx = 0;
        for (idx, _) in src_lc.match_indices(&needle_lc) {
            if idx >= last_idx
                && src.is_char_boundary(idx)
                && src.is_char_boundary(idx + needle.len())
            {
                out.push_str(&src[last_idx..idx]);
                out.push_str(replacement);
                last_idx = idx + needle.len();
            }
        }
        if last_idx == 0 {
            return None;
        }
        out.push_str(&src[last_idx..]);
        Some(out)
    }
}

pub fn compute_find_and_replace(
    project: &LoxoneProject,
    query: &str,
    replacement: &str,
    case_sensitive: bool,
    in_titles: bool,
    in_descriptions: bool,
    in_attributes: bool,
) -> (Vec<FindReplaceItem>, Option<ProjectEdit>) {
    let mut matches = Vec::new();
    if query.trim().is_empty() {
        return (matches, None);
    }

    for page in &project.pages {
        for block in &page.blocks {
            if block.is_ref_bar() || block.is_note() {
                continue;
            }
            if in_titles {
                if let Some(after) =
                    replace_substring(&block.title, query, replacement, case_sensitive)
                {
                    matches.push(FindReplaceItem {
                        uuid: block.uuid.clone(),
                        object_type: block.block_type.clone(),
                        location: page.title.clone(),
                        field_name: "Title".to_string(),
                        before: block.title.clone(),
                        after,
                    });
                }
            }
            if in_descriptions {
                if let Some(desc) = block.attr("Desc") {
                    if let Some(after) = replace_substring(desc, query, replacement, case_sensitive)
                    {
                        matches.push(FindReplaceItem {
                            uuid: block.uuid.clone(),
                            object_type: block.block_type.clone(),
                            location: page.title.clone(),
                            field_name: "Desc".to_string(),
                            before: desc.to_string(),
                            after,
                        });
                    }
                }
            }
            if in_attributes {
                for (k, v) in &block.raw_attributes {
                    if matches!(
                        k.as_str(),
                        "Title" | "Desc" | "U" | "Type" | "Px" | "Py" | "Px2" | "Py2"
                    ) {
                        continue;
                    }
                    if let Some(after) = replace_substring(v, query, replacement, case_sensitive) {
                        matches.push(FindReplaceItem {
                            uuid: block.uuid.clone(),
                            object_type: block.block_type.clone(),
                            location: page.title.clone(),
                            field_name: k.clone(),
                            before: v.clone(),
                            after,
                        });
                    }
                }
            }
        }
    }

    fn walk_tree(
        node: &TreeNode,
        loc: &str,
        query: &str,
        replacement: &str,
        case_sensitive: bool,
        in_titles: bool,
        in_descriptions: bool,
        in_attributes: bool,
        out: &mut Vec<FindReplaceItem>,
    ) {
        if !node.uuid.is_empty() && !node.control_type.ends_with("Caption") {
            if in_titles {
                if let Some(after) =
                    replace_substring(&node.title, query, replacement, case_sensitive)
                {
                    out.push(FindReplaceItem {
                        uuid: node.uuid.clone(),
                        object_type: node.control_type.clone(),
                        location: loc.to_string(),
                        field_name: "Title".to_string(),
                        before: node.title.clone(),
                        after,
                    });
                }
            }
            if in_descriptions {
                if let Some(desc) = node.attr("Desc") {
                    if let Some(after) = replace_substring(desc, query, replacement, case_sensitive)
                    {
                        out.push(FindReplaceItem {
                            uuid: node.uuid.clone(),
                            object_type: node.control_type.clone(),
                            location: loc.to_string(),
                            field_name: "Desc".to_string(),
                            before: desc.to_string(),
                            after,
                        });
                    }
                }
            }
            if in_attributes {
                for (k, v) in &node.raw_attributes {
                    if matches!(k.as_str(), "Title" | "Desc" | "U" | "Type") {
                        continue;
                    }
                    if let Some(after) = replace_substring(v, query, replacement, case_sensitive) {
                        out.push(FindReplaceItem {
                            uuid: node.uuid.clone(),
                            object_type: node.control_type.clone(),
                            location: loc.to_string(),
                            field_name: k.clone(),
                            before: v.clone(),
                            after,
                        });
                    }
                }
            }
        }
        let child_loc = if node.title.is_empty() {
            loc
        } else {
            &node.title
        };
        for c in &node.children {
            walk_tree(
                c,
                child_loc,
                query,
                replacement,
                case_sensitive,
                in_titles,
                in_descriptions,
                in_attributes,
                out,
            );
        }
    }

    for n in &project.periphery_tree {
        walk_tree(
            n,
            "Periphery",
            query,
            replacement,
            case_sensitive,
            in_titles,
            in_descriptions,
            in_attributes,
            &mut matches,
        );
    }
    for n in &project.system_tree {
        walk_tree(
            n,
            "System",
            query,
            replacement,
            case_sensitive,
            in_titles,
            in_descriptions,
            in_attributes,
            &mut matches,
        );
    }

    let batch = if matches.is_empty() {
        None
    } else {
        let edits = matches
            .iter()
            .map(|m| ProjectEdit::SetControlAttr {
                control_uuid: m.uuid.clone(),
                attr_name: m.field_name.clone(),
                value: Some(m.after.clone()),
            })
            .collect();
        Some(ProjectEdit::Batch(edits))
    };

    (matches, batch)
}
