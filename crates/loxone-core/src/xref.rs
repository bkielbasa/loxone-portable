use std::collections::HashMap;

use crate::model::{ConnectorPin, LoxoneProject, RawNode, TreeNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectLocation {
    DocumentRoot,
    Category,
    Place,
    Page { page_index: usize },
    PageBlock { page_index: usize, block_index: usize, page_uuid: String, page_title: String },
    TreeNode { section: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectSummary {
    pub uuid: String,
    pub title: String,
    pub object_type: String,
    pub location: ObjectLocation,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinLocation {
    pub pin_uuid: String,
    pub pin_key: String,
    pub owner_uuid: String,
    pub owner_title: String,
    pub owner_type: String,
    pub page_index: Option<usize>,
    pub page_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingWireTarget {
    pub target_pin_uuid: String,
    pub target_pin_key: String,
    pub target_block_uuid: String,
    pub target_block_title: String,
    pub target_page_index: usize,
    pub target_page_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossRefKind {
    DirectRef,
    ExtraDeviceRef,
    WireFromPin { source_pin_key: String, target_pin_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRefUsage {
    pub page_index: usize,
    pub page_uuid: String,
    pub page_title: String,
    pub block_uuid: String,
    pub block_title: String,
    pub block_type: String,
    pub kind: CrossRefKind,
}

#[derive(Debug, Clone, Default)]
pub struct XrefIndex {
    pub objects_by_uuid: HashMap<String, ObjectSummary>,
    pub pins_by_uuid: HashMap<String, PinLocation>,
    pub outgoing_wires_by_pin_uuid: HashMap<String, Vec<OutgoingWireTarget>>,
    pub usages_by_target_uuid: HashMap<String, Vec<CrossRefUsage>>,
}

impl XrefIndex {
    pub fn build(project: &LoxoneProject) -> Self {
        let mut index = XrefIndex::default();

        if !project.metadata.uuid.is_empty() {
            index.objects_by_uuid.insert(
                project.metadata.uuid.clone(),
                ObjectSummary {
                    uuid: project.metadata.uuid.clone(),
                    title: project.metadata.title.clone(),
                    object_type: "Document".to_string(),
                    location: ObjectLocation::DocumentRoot,
                    room_uuid: None,
                    category_uuid: None,
                },
            );
        }

        for cat in project.categories.values() {
            index.objects_by_uuid.insert(
                cat.uuid.clone(),
                ObjectSummary {
                    uuid: cat.uuid.clone(),
                    title: cat.title.clone(),
                    object_type: "Category".to_string(),
                    location: ObjectLocation::Category,
                    room_uuid: None,
                    category_uuid: Some(cat.uuid.clone()),
                },
            );
        }

        for place in project.places.values() {
            index.objects_by_uuid.insert(
                place.uuid.clone(),
                ObjectSummary {
                    uuid: place.uuid.clone(),
                    title: place.title.clone(),
                    object_type: "Place".to_string(),
                    location: ObjectLocation::Place,
                    room_uuid: Some(place.uuid.clone()),
                    category_uuid: None,
                },
            );
        }

        for node in &project.periphery_tree {
            Self::index_tree_node(&mut index, node, "Periphery");
        }
        for node in &project.system_tree {
            Self::index_tree_node(&mut index, node, "System");
        }

        for (page_index, page) in project.pages.iter().enumerate() {
            index.objects_by_uuid.insert(
                page.uuid.clone(),
                ObjectSummary {
                    uuid: page.uuid.clone(),
                    title: page.title.clone(),
                    object_type: "Page".to_string(),
                    location: ObjectLocation::Page { page_index },
                    room_uuid: page.room_uuid.clone(),
                    category_uuid: page.category_uuid.clone(),
                },
            );

            for (block_index, block) in page.blocks.iter().enumerate() {
                let effective_room = block.room_uuid.clone().or_else(|| page.room_uuid.clone());
                let effective_cat = block
                    .category_uuid
                    .clone()
                    .or_else(|| page.category_uuid.clone());

                index.objects_by_uuid.insert(
                    block.uuid.clone(),
                    ObjectSummary {
                        uuid: block.uuid.clone(),
                        title: block.title.clone(),
                        object_type: block.block_type.clone(),
                        location: ObjectLocation::PageBlock {
                            page_index,
                            block_index,
                            page_uuid: page.uuid.clone(),
                            page_title: page.title.clone(),
                        },
                        room_uuid: effective_room,
                        category_uuid: effective_cat,
                    },
                );

                Self::index_pins(
                    &mut index,
                    &block.pins,
                    &block.uuid,
                    &block.title,
                    &block.block_type,
                    Some(page_index),
                    Some(&page.title),
                );
                Self::index_raw_nodes(
                    &mut index,
                    &block.extra_children,
                    &block.uuid,
                    &block.title,
                    &block.block_type,
                    Some(page_index),
                    Some(&page.title),
                );
            }
        }

        // Second pass over page blocks to record references and wire links
        for (page_index, page) in project.pages.iter().enumerate() {
            for block in &page.blocks {
                if let Some(ref_uuid) = &block.ref_uuid {
                    index
                        .usages_by_target_uuid
                        .entry(ref_uuid.clone())
                        .or_default()
                        .push(CrossRefUsage {
                            page_index,
                            page_uuid: page.uuid.clone(),
                            page_title: page.title.clone(),
                            block_uuid: block.uuid.clone(),
                            block_title: block.title.clone(),
                            block_type: block.block_type.clone(),
                            kind: CrossRefKind::DirectRef,
                        });
                }

                for extra_uuid in &block.extra_ref_uuids {
                    index
                        .usages_by_target_uuid
                        .entry(extra_uuid.clone())
                        .or_default()
                        .push(CrossRefUsage {
                            page_index,
                            page_uuid: page.uuid.clone(),
                            page_title: page.title.clone(),
                            block_uuid: block.uuid.clone(),
                            block_title: block.title.clone(),
                            block_type: block.block_type.clone(),
                            kind: CrossRefKind::ExtraDeviceRef,
                        });
                }

                for pin in &block.pins {
                    for link in &pin.incoming_links {
                        index
                            .outgoing_wires_by_pin_uuid
                            .entry(link.source_pin_uuid.clone())
                            .or_default()
                            .push(OutgoingWireTarget {
                                target_pin_uuid: pin.uuid.clone(),
                                target_pin_key: pin.key.clone(),
                                target_block_uuid: block.uuid.clone(),
                                target_block_title: block.title.clone(),
                                target_page_index: page_index,
                                target_page_title: page.title.clone(),
                            });

                        if let Some(src_pin) = index.pins_by_uuid.get(&link.source_pin_uuid).cloned()
                        {
                            index
                                .usages_by_target_uuid
                                .entry(src_pin.owner_uuid.clone())
                                .or_default()
                                .push(CrossRefUsage {
                                    page_index,
                                    page_uuid: page.uuid.clone(),
                                    page_title: page.title.clone(),
                                    block_uuid: block.uuid.clone(),
                                    block_title: block.title.clone(),
                                    block_type: block.block_type.clone(),
                                    kind: CrossRefKind::WireFromPin {
                                        source_pin_key: src_pin.pin_key,
                                        target_pin_key: pin.key.clone(),
                                    },
                                });
                        }
                    }
                }
            }
        }

        index
    }

    pub fn usages_for(&self, target_uuid: &str) -> &[CrossRefUsage] {
        self.usages_by_target_uuid
            .get(target_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    fn index_tree_node(index: &mut XrefIndex, node: &TreeNode, section_label: &str) {
        if !node.uuid.is_empty() {
            index.objects_by_uuid.insert(
                node.uuid.clone(),
                ObjectSummary {
                    uuid: node.uuid.clone(),
                    title: node.title.clone(),
                    object_type: node.control_type.clone(),
                    location: ObjectLocation::TreeNode {
                        section: section_label.to_string(),
                    },
                    room_uuid: node.room_uuid.clone(),
                    category_uuid: node.category_uuid.clone(),
                },
            );
        }
        Self::index_pins(
            index,
            &node.pins,
            &node.uuid,
            &node.title,
            &node.control_type,
            None,
            None,
        );
        Self::index_raw_nodes(
            index,
            &node.extra_children,
            &node.uuid,
            &node.title,
            &node.control_type,
            None,
            None,
        );
        for child in &node.children {
            Self::index_tree_node(index, child, section_label);
        }
    }

    fn index_pins(
        index: &mut XrefIndex,
        pins: &[ConnectorPin],
        owner_uuid: &str,
        owner_title: &str,
        owner_type: &str,
        page_index: Option<usize>,
        page_title: Option<&str>,
    ) {
        for pin in pins {
            if !pin.uuid.is_empty() {
                index.pins_by_uuid.insert(
                    pin.uuid.clone(),
                    PinLocation {
                        pin_uuid: pin.uuid.clone(),
                        pin_key: pin.key.clone(),
                        owner_uuid: owner_uuid.to_string(),
                        owner_title: owner_title.to_string(),
                        owner_type: owner_type.to_string(),
                        page_index,
                        page_title: page_title.map(str::to_string),
                    },
                );
            }
        }
    }

    fn index_raw_nodes(
        index: &mut XrefIndex,
        raw_nodes: &[RawNode],
        owner_uuid: &str,
        owner_title: &str,
        owner_type: &str,
        page_index: Option<usize>,
        page_title: Option<&str>,
    ) {
        for raw in raw_nodes {
            let (cur_uuid, cur_title, cur_type) = if raw.tag == "C" {
                (
                    raw.attr("U").unwrap_or(owner_uuid),
                    raw.attr("Title").unwrap_or(owner_title),
                    raw.attr("Type").unwrap_or(owner_type),
                )
            } else {
                (owner_uuid, owner_title, owner_type)
            };

            if raw.tag == "Co" {
                let pin_uuid = raw.attr("U").unwrap_or_default();
                let pin_key = raw.attr("K").unwrap_or_default();
                if !pin_uuid.is_empty() {
                    index.pins_by_uuid.insert(
                        pin_uuid.to_string(),
                        PinLocation {
                            pin_uuid: pin_uuid.to_string(),
                            pin_key: pin_key.to_string(),
                            owner_uuid: cur_uuid.to_string(),
                            owner_title: cur_title.to_string(),
                            owner_type: cur_type.to_string(),
                            page_index,
                            page_title: page_title.map(str::to_string),
                        },
                    );
                }
            }

            Self::index_raw_nodes(
                index,
                &raw.children,
                cur_uuid,
                cur_title,
                cur_type,
                page_index,
                page_title,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::parser::load_loxone_file;

    #[test]
    fn test_klimczaki_xref_counts_and_usages() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        // All 5,433 <Co> pins across pages and periphery/system trees are indexed
        assert_eq!(project.xref.pins_by_uuid.len(), 5433);

        // Verify a peripheral referenced by an InputRef on Page 0 ("Alarm 1") has recorded page usages
        let page0 = &project.pages[0];
        let first_input_ref = page0
            .blocks
            .iter()
            .find(|b| b.block_type == "InputRef" && b.ref_uuid.is_some())
            .expect("Page 0 should have an InputRef");
        let target_uuid = first_input_ref.ref_uuid.as_deref().unwrap();

        let usages = project.xref.usages_for(target_uuid);
        assert!(
            !usages.is_empty(),
            "Peripheral {} referenced by InputRef should have cross-reference usages",
            target_uuid
        );
        assert!(usages.iter().any(|u| u.block_uuid == first_input_ref.uuid));
    }
}
