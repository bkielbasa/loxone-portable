//! Project-administration features exercised against the bundled Sample project.

use loxone_core::export::{page_to_svg, project_documentation_html};
use loxone_core::parser::parse_loxone_xml;
use loxone_core::project_admin::{
    calendar_entries, find_first_by_type, find_nodes_by_type, granted_permission_refs,
    operating_modes, set_permission_edit,
};
use loxone_core::writer::{apply_edit, serialize_loxone_xml, ProjectEdit};

fn sample() -> Option<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Sample.Loxone");
    std::fs::read(path)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).to_string())
}

#[test]
fn sample_admin_roundtrip() {
    let Some(xml) = sample() else { return };
    let mut p = parse_loxone_xml(&xml).expect("parse sample");
    assert!(!find_nodes_by_type(&p, "User").is_empty());
    assert!(!find_nodes_by_type(&p, "Permission").is_empty());
    assert!(operating_modes(&p).len() > 5);
    assert!(!calendar_entries(&p).is_empty());
    assert_eq!(p.metadata.doc_attr("Timezone"), Some("Europe/Warsaw"));

    let doc = p.metadata.uuid.clone();
    apply_edit(
        &mut p,
        &ProjectEdit::SetControlAttr {
            control_uuid: doc,
            attr_name: "Town".into(),
            value: Some("Edited Town".into()),
        },
    )
    .unwrap();
    let g = find_first_by_type(&p, "UserGroup").unwrap().clone();
    let perm = find_first_by_type(&p, "Permission").unwrap().uuid.clone();
    let had = granted_permission_refs(&g).contains(&perm);
    apply_edit(&mut p, &set_permission_edit(&g, &perm, !had)).unwrap();

    let out = serialize_loxone_xml(&p).unwrap();
    let re = parse_loxone_xml(&out).expect("reparse");
    assert_eq!(re.metadata.town, "Edited Town");
    assert_eq!(re.metadata.doc_attr("Timezone"), Some("Europe/Warsaw"));
    let g2 = find_first_by_type(&re, "UserGroup").unwrap();
    assert_eq!(granted_permission_refs(g2).contains(&perm), !had);

    let page = re.pages.iter().find(|pg| !pg.blocks.is_empty()).expect("a page");
    let svg = page_to_svg(&re, page);
    assert!(svg.starts_with("<svg") || svg.starts_with("<?xml"));
    assert!(svg.contains("</svg>"));
    let html = project_documentation_html(&re);
    assert!(html.contains("Edited Town"));
}
