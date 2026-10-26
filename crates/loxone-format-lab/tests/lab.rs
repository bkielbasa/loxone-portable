use std::path::PathBuf;

use loxone_format_lab::corpus::load_file;
use loxone_format_lab::diff::{diff_files, DiffOptions};
use loxone_format_lab::gen::{generate_rust, GenOptions};
use loxone_format_lab::report::{render_check, render_markdown, CheckReport, Coverage, MarkdownOptions};
use loxone_format_lab::stats::scan;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn test_load_fixture_reports_pages_blocks_and_pins() {
    let loaded = load_file(&fixture("base.Loxone")).expect("base fixture should load");
    assert_eq!(loaded.summary.title, "Lab House");
    assert_eq!(loaded.summary.pages, 1);
    assert_eq!(loaded.summary.blocks, 3);
    assert_eq!(loaded.summary.pins, 7);
    assert_eq!(loaded.summary.control_list_version, "274");
}

#[test]
fn test_scan_aggregates_attributes_pins_and_unknown_types() {
    let stats = scan(&[fixture("base.Loxone")]).expect("scan should succeed");
    assert_eq!(stats.totals.files, 1);
    assert_eq!(stats.totals.blocks, 3);
    assert!(stats.errors.is_empty());

    let light = stats
        .types
        .get("LightController2")
        .expect("LightController2 should be observed");
    assert_eq!(light.occurrences, 1);
    assert_eq!(light.attributes["Title"].count, 1);
    assert_eq!(light.attributes["Nio"].examples, vec!["4".to_string()]);
    assert_eq!(light.pins["I1"].defaults.examples, vec!["0".to_string()]);
    assert_eq!(light.pins["I1"].incoming_links, 1);
    assert!(light.pins["AQ1"].seen_as_wire_source);

    let coverage = Coverage::build(&stats);
    assert!(
        coverage
            .unknown_types
            .iter()
            .any(|item| item.type_name == "MyCustomBlock"),
        "MyCustomBlock should be reported as missing a schema"
    );
    assert!(
        !coverage
            .unknown_types
            .iter()
            .any(|item| item.type_name == "Page"),
        "structural types must not be reported as unknown blocks"
    );

    let markdown = render_markdown(&stats, &MarkdownOptions::default());
    assert!(markdown.contains("MyCustomBlock"));
    assert!(markdown.contains("LightController2"));
}

#[test]
fn test_diff_finds_attribute_pin_and_child_changes() {
    let report = diff_files(
        &fixture("base.Loxone"),
        &fixture("edited.Loxone"),
        &DiffOptions::default(),
    )
    .expect("diff should succeed");

    assert_eq!(report.summary.added, 1, "new Switch block should be added");
    assert_eq!(
        report.summary.removed, 1,
        "MyCustomBlock should be removed from the edited save"
    );
    assert!(
        !report
            .changed
            .iter()
            .any(|node| node.brief.uuid.as_deref() == Some("doc-1")),
        "ignored SL change must not produce a document diff"
    );

    let light = report
        .changed
        .iter()
        .find(|node| node.brief.uuid.as_deref() == Some("blk-light"))
        .expect("LightController2 should have changes");
    let fields: Vec<&str> = light
        .changes
        .iter()
        .map(|change| change.field.as_str())
        .collect();
    assert!(fields.contains(&"Title"));
    assert!(fields.contains(&"Nio"));
    assert!(fields.contains(&"TE"));
    assert!(fields.contains(&"Co[I1].Def"));
    assert!(fields.contains(&"Co[I2]"), "fields: {fields:?}");
    assert!(fields.contains(&"Co[Step].Def"));
    assert!(fields.contains(&"IoData.Pr"));
    assert!(fields.contains(&"HCL"), "fields: {fields:?}");
    assert!(fields.contains(&"PSD"), "fields: {fields:?}");
    assert!(fields.contains(&"LoggerMailer"), "fields: {fields:?}");
}

#[test]
fn test_check_reports_actionable_gaps() {
    let stats = scan(&[fixture("base.Loxone")]).expect("scan should succeed");
    let report = CheckReport::build(&stats, Coverage::build(&stats));
    assert!(report.has_issues());
    assert!(report.issue_count() > 0);
    assert!(report
        .coverage
        .unknown_types
        .iter()
        .any(|item| item.type_name == "MyCustomBlock"));
    let rendered = render_check(&report);
    assert!(rendered.contains("Actionable gaps:"));
    assert!(rendered.contains("## Universal attributes not declared in schemas"));
    assert!(rendered.contains("MyCustomBlock"));
}

#[test]
fn test_generate_rust_includes_known_types_with_flag() {
    let stats = scan(&[fixture("base.Loxone")]).expect("scan should succeed");
    let code = generate_rust(
        &stats,
        &GenOptions {
            types: Vec::new(),
            include_known: true,
            include_non_block: false,
            include_generic_attributes: false,
        },
    )
    .expect("code generation should succeed");

    assert!(code.contains("\"MyCustomBlock\""));
    assert!(code.contains("\"LightController2\""));
}

#[test]
fn test_generate_rust_for_unknown_type() {
    let stats = scan(&[fixture("base.Loxone")]).expect("scan should succeed");
    let code = generate_rust(
        &stats,
        &GenOptions {
            types: vec!["MyCustomBlock".to_string()],
            include_known: false,
            include_non_block: false,
            include_generic_attributes: false,
        },
    )
    .expect("code generation should succeed");

    assert!(code.contains("schema!("));
    assert!(code.contains("\"MyCustomBlock\""));
    assert!(code.contains("attr!(\"Mode\""));
    assert!(code.contains("attr!(\"Threshold\""));
    assert!(code.contains("pin_in!(\"I1\""));
    assert!(code.contains("pin_out!(\"Q1\""));
    assert!(code.contains("ChildConfigKind::Display"));
}
