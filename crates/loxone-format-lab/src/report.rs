//! Markdown reports and schema-coverage checks built from corpus statistics.

use std::collections::{BTreeMap, BTreeSet};

use loxone_core::block_schema::{all_block_schemas, lookup_block_schema, BlockSchema, ChildConfigKind};
use serde::Serialize;

use crate::dom::is_structural_type;
use crate::stats::{AttrStats, CorpusStats, PinStats, Totals};

/// Attributes Loxone uses on nearly every control. They are not interesting as
/// "missing schema" findings, but are still shown in the detailed report.
const UNIVERSAL_ATTRIBUTES: &[&str] = &[
    "U", "Title", "Type", "V", "WF", "Nio", "Cl", "Px", "Py", "Px2", "Py2", "Ref", "SL",
    "DateS", "CDate", "LtE",
];

#[derive(Debug, Serialize)]
pub struct Coverage {
    pub schema_type_count: usize,
    pub observed_type_count: usize,
    pub known_observed_count: usize,
    pub unknown_types: Vec<TypeCount>,
    pub unobserved_schema_types: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TypeCount {
    pub type_name: String,
    pub occurrences: usize,
    pub root_sections: BTreeMap<String, usize>,
    /// True when the type looks like a function block or periphery device
    /// rather than metadata (icon tables, captions, document sections).
    pub schema_candidate: bool,
}

impl Coverage {
    pub fn build(stats: &CorpusStats) -> Self {
        let schema_types: BTreeSet<String> = all_block_schemas()
            .map(|schema| schema.type_name.to_string())
            .collect();
        // Document/Page/Place/... form the skeleton and are not block schemas,
        // so exclude them from block coverage numbers.
        let observed_types: BTreeSet<String> = stats
            .types
            .keys()
            .filter(|ty| !is_structural_type(ty))
            .cloned()
            .collect();

        let mut unknown_types: Vec<TypeCount> = observed_types
            .iter()
            .filter(|ty| !schema_types.contains(*ty))
            .map(|ty| {
                let type_stats = &stats.types[ty];
                TypeCount {
                    type_name: ty.clone(),
                    occurrences: type_stats.occurrences,
                    root_sections: type_stats.root_sections.clone(),
                    schema_candidate: is_schema_candidate(ty, type_stats),
                }
            })
            .collect();
        unknown_types.sort_by(|a, b| {
            b.schema_candidate
                .cmp(&a.schema_candidate)
                .then_with(|| b.occurrences.cmp(&a.occurrences))
                .then_with(|| a.type_name.cmp(&b.type_name))
        });

        let unobserved_schema_types: Vec<String> = schema_types
            .difference(&observed_types)
            .cloned()
            .collect();

        Self {
            schema_type_count: schema_types.len(),
            observed_type_count: observed_types.len(),
            known_observed_count: observed_types
                .iter()
                .filter(|ty| schema_types.contains(*ty))
                .count(),
            unknown_types,
            unobserved_schema_types,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarkdownOptions {
    pub min_count: usize,
    pub max_types: usize,
    pub examples: usize,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            min_count: 1,
            max_types: 0,
            examples: 4,
        }
    }
}

pub fn render_markdown(stats: &CorpusStats, options: &MarkdownOptions) -> String {
    let coverage = Coverage::build(stats);
    let mut out = String::new();

    out.push_str("# Loxone format corpus report\n\n");
    out.push_str(&format!(
        "Scanned **{}** file(s): {} pages, {} page blocks, {} connector pins, {} typed nodes, {} total XML nodes.\n\n",
        stats.totals.files,
        stats.totals.pages,
        stats.totals.blocks,
        stats.totals.pins,
        stats.totals.typed_nodes,
        stats.totals.nodes
    ));
    out.push_str(&format!(
        "Schema coverage: **{}/{}** observed types are known ({:.1}%); {} known schema types were not observed.\n\n",
        coverage.known_observed_count,
        coverage.observed_type_count,
        percent(coverage.known_observed_count, coverage.observed_type_count),
        coverage.unobserved_schema_types.len()
    ));

    out.push_str("## Files\n\n");
    out.push_str("| File | Title | Config | Pages | Blocks | Pins | Nodes |\n");
    out.push_str("| :--- | :--- | :--- | ---: | ---: | ---: | ---: |\n");
    for file in &stats.files {
        out.push_str(&format!(
            "| `{}` | {} | `{}` | {} | {} | {} | {} |\n",
            md_escape(&file.path),
            md_escape(empty_or(&file.title, "—")),
            md_escape(empty_or(&file.config_version, "—")),
            file.pages,
            file.blocks,
            file.pins,
            file.nodes
        ));
    }

    if !stats.errors.is_empty() {
        out.push_str("\n## Scan errors\n\n");
        for error in &stats.errors {
            out.push_str(&format!("- {}\n", md_escape(error)));
        }
    }

    if !coverage.unknown_types.is_empty() {
        out.push_str("\n## Types with no schema\n\n");
        let blocks: Vec<&TypeCount> = coverage
            .unknown_types
            .iter()
            .filter(|item| item.schema_candidate)
            .collect();
        let other: Vec<&TypeCount> = coverage
            .unknown_types
            .iter()
            .filter(|item| !item.schema_candidate)
            .collect();

        if !blocks.is_empty() {
            out.push_str("### Block / periphery types (actionable)\n\n");
            render_type_table(&mut out, &blocks);
        }
        if !other.is_empty() {
            out.push_str("\n### Other XML types (metadata, captions, sections)\n\n");
            out.push_str(
                "These are not block schemas; they are reported for completeness.\n\n",
            );
            render_type_table(&mut out, &other);
        }
    }

    let mut types: Vec<(&String, &crate::stats::TypeStats)> = stats
        .types
        .iter()
        .filter(|(_, type_stats)| type_stats.occurrences >= options.min_count)
        .collect();
    types.sort_by(|a, b| {
        b.1.occurrences
            .cmp(&a.1.occurrences)
            .then_with(|| a.0.cmp(b.0))
    });
    if options.max_types > 0 {
        types.truncate(options.max_types);
    }

    out.push_str("\n## Observed types\n\n");
    for (type_name, type_stats) in types {
        let schema = lookup_block_schema(type_name);
        let schema_title = schema
            .map(|s| s.display_name_en)
            .unwrap_or("(no schema)");
        out.push_str(&format!(
            "### `{}` — {} ({})\n\n",
            md_escape(type_name),
            md_escape(schema_title),
            type_stats.occurrences
        ));
        out.push_str(&format!(
            "- Schema: {}\n",
            if schema.is_some() { "yes" } else { "**missing**" }
        ));
        out.push_str(&format!(
            "- Contexts: {}\n",
            context_summary(&type_stats.root_sections)
        ));
        out.push_str(&format!(
            "- Parents: {}\n",
            context_summary(&type_stats.parent_types)
        ));
        if !type_stats.titles.is_empty() {
            out.push_str(&format!(
                "- Example titles: {}\n",
                type_stats
                    .titles
                    .iter()
                    .take(options.examples)
                    .map(|t| format!("`{}`", md_escape(t)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        out.push('\n');

        if !type_stats.attributes.is_empty() {
            out.push_str("| Attribute | Fill | Kind | Distinct | Schema | Observed values |\n");
            out.push_str("| :--- | ---: | :--- | ---: | :--- | :--- |\n");
            let mut attrs: Vec<(&String, &AttrStats)> = type_stats.attributes.iter().collect();
            attrs.sort_by(|a, b| b.1.count.cmp(&a.1.count).then_with(|| a.0.cmp(b.0)));
            for (name, attr) in attrs {
                let schema_known = schema
                    .map(|s| s.find_attr(name).is_some())
                    .unwrap_or(false);
                out.push_str(&format!(
                    "| `{}` | {:.0}% | {} | {} | {} | {} |\n",
                    md_escape(name),
                    percent(attr.count, type_stats.occurrences),
                    attr.inferred_kind(),
                    attr.distinct,
                    if schema_known { "yes" } else { "**no**" },
                    example_values(attr, options.examples)
                ));
            }
            out.push('\n');
        }

        if !type_stats.pins.is_empty() {
            out.push_str("| Pin | Count | Direction | Defaults | Links | Schema |\n");
            out.push_str("| :--- | ---: | :--- | :--- | ---: | :--- |\n");
            let mut pins: Vec<(&String, &PinStats)> = type_stats.pins.iter().collect();
            pins.sort_by(|a, b| b.1.occurrences.cmp(&a.1.occurrences).then_with(|| a.0.cmp(b.0)));
            for (key, pin) in pins {
                let schema_state = match schema {
                    Some(s) => {
                        let param = s.find_pin_param(key).is_some();
                        let doc = s.find_pin_doc(key).is_some();
                        match (param, doc) {
                            (true, true) => "param+doc",
                            (true, false) => "param",
                            (false, true) => "doc",
                            (false, false) => "**no**",
                        }
                    }
                    None => "**no schema**",
                };
                out.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} | {} |\n",
                    md_escape(key),
                    pin.occurrences,
                    if pin.is_probably_output(key) {
                        "output"
                    } else {
                        "input"
                    },
                    example_values(&pin.defaults, options.examples),
                    pin.incoming_links,
                    schema_state
                ));
            }
            out.push('\n');
        }

        if !type_stats.children.is_empty() {
            let mut children: Vec<(&String, usize)> = type_stats
                .children
                .iter()
                .map(|(tag, child)| (tag, child.count))
                .collect();
            children.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            out.push_str("Child elements: ");
            out.push_str(
                &children
                    .iter()
                    .map(|(tag, count)| format!("`{}` ×{}", md_escape(tag), count))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push_str("\n\n");
        }
    }

    if !coverage.unobserved_schema_types.is_empty() {
        out.push_str("## Known schema types not observed\n\n");
        out.push_str(
            &coverage
                .unobserved_schema_types
                .iter()
                .map(|ty| format!("`{}`", md_escape(ty)))
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str("\n");
    }

    out
}

#[derive(Debug, Serialize)]
pub struct CheckReport {
    pub totals: Totals,
    pub coverage: Coverage,
    pub errors: Vec<String>,
    pub unknown_attributes: Vec<UnknownField>,
    pub unknown_pins: Vec<UnknownField>,
    pub unknown_child_tags: Vec<UnknownField>,
    pub unused_schema_types: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct UnknownField {
    pub type_name: String,
    pub field: String,
    pub occurrences: usize,
    pub type_occurrences: usize,
    pub fill_percent: f64,
    pub inferred_kind: Option<String>,
    pub examples: Vec<String>,
    pub universal: bool,
}

impl CheckReport {
    pub fn build(stats: &CorpusStats, coverage: Coverage) -> Self {
        let mut unknown_attributes = Vec::new();
        let mut unknown_pins = Vec::new();
        let mut unknown_child_tags = Vec::new();

        for (type_name, type_stats) in &stats.types {
            // The document skeleton is handled by the parser, not by block schemas.
            if is_structural_type(type_name) {
                continue;
            }
            let Some(schema) = lookup_block_schema(type_name) else {
                continue;
            };
            for (name, attr) in &type_stats.attributes {
                if schema.find_attr(name).is_some() {
                    continue;
                }
                unknown_attributes.push(unknown_field(
                    type_name,
                    name,
                    attr.count,
                    type_stats.occurrences,
                    attr.inferred_kind(),
                    &attr.examples,
                    UNIVERSAL_ATTRIBUTES.contains(&name.as_str()),
                ));
            }
            for (key, pin) in &type_stats.pins {
                if schema.find_pin_param(key).is_some() || schema.find_pin_doc(key).is_some() {
                    continue;
                }
                unknown_pins.push(unknown_field(
                    type_name,
                    key,
                    pin.occurrences,
                    type_stats.occurrences,
                    if pin.is_probably_output(key) {
                        "output"
                    } else {
                        "input"
                    },
                    &pin.defaults.examples,
                    false,
                ));
            }
            for (tag, child) in &type_stats.children {
                if tag == "C" || child_config_matches(schema, tag) {
                    continue;
                }
                unknown_child_tags.push(unknown_field(
                    type_name,
                    tag,
                    child.count,
                    type_stats.occurrences,
                    "element",
                    &[],
                    false,
                ));
            }
        }

        unknown_attributes.sort_by(compare_unknown);
        unknown_pins.sort_by(compare_unknown);
        unknown_child_tags.sort_by(compare_unknown);

        Self {
            totals: stats.totals.clone(),
            unused_schema_types: coverage.unobserved_schema_types.clone(),
            coverage,
            errors: stats.errors.clone(),
            unknown_attributes,
            unknown_pins,
            unknown_child_tags,
        }
    }

    pub fn issue_count(&self) -> usize {
        let actionable_attrs = self
            .unknown_attributes
            .iter()
            .filter(|field| !field.universal)
            .count();
        let missing_block_types = self
            .coverage
            .unknown_types
            .iter()
            .filter(|item| item.schema_candidate)
            .count();
        missing_block_types
            + actionable_attrs
            + self.unknown_pins.len()
            + self.unknown_child_tags.len()
    }

    pub fn has_issues(&self) -> bool {
        self.issue_count() > 0
    }
}

pub fn render_check(report: &CheckReport) -> String {
    let mut out = String::new();
    out.push_str("# Schema coverage check\n\n");
    out.push_str(&format!(
        "Files: {} · page blocks: {} · pins: {} · types observed: {} · schema types known: {}\n\n",
        report.totals.files,
        report.totals.blocks,
        report.totals.pins,
        report.coverage.observed_type_count,
        report.coverage.known_observed_count
    ));

    let unknown_blocks: Vec<&TypeCount> = report
        .coverage
        .unknown_types
        .iter()
        .filter(|item| item.schema_candidate)
        .collect();
    let unknown_other: Vec<&TypeCount> = report
        .coverage
        .unknown_types
        .iter()
        .filter(|item| !item.schema_candidate)
        .collect();
    let actionable_attrs = report
        .unknown_attributes
        .iter()
        .filter(|field| !field.universal)
        .count();
    out.push_str(&format!(
        "Actionable gaps: {} block type(s), {} attribute(s), {} pin(s), {} child element(s).\n\n",
        unknown_blocks.len(),
        actionable_attrs,
        report.unknown_pins.len(),
        report.unknown_child_tags.len()
    ));

    if unknown_blocks.is_empty() {
        out.push_str("✓ Every observed block/periphery type has a schema.\n\n");
    } else {
        out.push_str(&format!(
            "## ✗ Unknown block/periphery types ({})\n\n",
            unknown_blocks.len()
        ));
        for item in &unknown_blocks {
            out.push_str(&format!(
                "- `{}` — {} occurrence(s), {}\n",
                md_escape(&item.type_name),
                item.occurrences,
                context_summary(&item.root_sections)
            ));
        }
        out.push('\n');
    }

    if !unknown_other.is_empty() {
        out.push_str(&format!(
            "## Non-block XML types without schemas ({})\n\n",
            unknown_other.len()
        ));
        out.push_str(
            "Metadata, captions, and document sections; these do not need block schemas.\n\n",
        );
        out.push_str(
            &unknown_other
                .iter()
                .map(|item| {
                    format!(
                        "`{}` ×{}",
                        md_escape(&item.type_name),
                        item.occurrences
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str("\n\n");
    }

    if !report.unknown_attributes.is_empty() {
        let actionable: Vec<&UnknownField> = report
            .unknown_attributes
            .iter()
            .filter(|field| !field.universal)
            .collect();
        let universal: Vec<&UnknownField> = report
            .unknown_attributes
            .iter()
            .filter(|field| field.universal)
            .collect();

        if !actionable.is_empty() {
            out.push_str(&format!(
                "## Attributes missing from schemas ({} actionable)\n\n",
                actionable.len()
            ));
            for field in &actionable {
                out.push_str(&format!(
                    "- `{}` on `{}` — {:.0}% ({}/{}) · {}{}",
                    md_escape(&field.field),
                    md_escape(&field.type_name),
                    field.fill_percent,
                    field.occurrences,
                    field.type_occurrences,
                    field.inferred_kind.as_deref().unwrap_or("unknown"),
                    if field.universal { " · universal" } else { "" }
                ));
                if !field.examples.is_empty() {
                    out.push_str(&format!(
                        " · examples: {}",
                        field
                            .examples
                            .iter()
                            .take(3)
                            .map(|v| format!("`{}`", md_escape(v)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                out.push('\n');
            }
            out.push('\n');
        }

        if !universal.is_empty() {
            out.push_str(&format!(
                "## Universal attributes not declared in schemas ({})\n\n",
                universal.len()
            ));
            out.push_str(
                "These attributes appear on nearly every control and are not counted as schema gaps.\n\n",
            );
            let mut by_attr: BTreeMap<&str, (BTreeSet<&str>, usize)> = BTreeMap::new();
            for field in &universal {
                let entry = by_attr.entry(field.field.as_str()).or_default();
                entry.0.insert(field.type_name.as_str());
                entry.1 += field.occurrences;
            }
            let mut by_attr: Vec<(&str, (BTreeSet<&str>, usize))> = by_attr.into_iter().collect();
            by_attr.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then_with(|| a.0.cmp(b.0)));
            out.push_str("| Attribute | Types | Occurrences |\n");
            out.push_str("| :--- | ---: | ---: |\n");
            for (name, (types, occurrences)) in by_attr {
                out.push_str(&format!(
                    "| `{}` | {} | {} |\n",
                    md_escape(name),
                    types.len(),
                    occurrences
                ));
            }
            out.push('\n');
        }
    }

    if !report.unknown_pins.is_empty() {
        out.push_str(&format!(
            "## Pins missing from schemas ({})\n\n",
            report.unknown_pins.len()
        ));
        for field in &report.unknown_pins {
            out.push_str(&format!(
                "- `{}` on `{}` — {} occurrence(s) · {}\n",
                md_escape(&field.field),
                md_escape(&field.type_name),
                field.occurrences,
                field.inferred_kind.as_deref().unwrap_or("unknown")
            ));
        }
        out.push('\n');
    }

    if !report.unknown_child_tags.is_empty() {
        out.push_str(&format!(
            "## Child elements missing from schemas ({})\n\n",
            report.unknown_child_tags.len()
        ));
        for field in &report.unknown_child_tags {
            out.push_str(&format!(
                "- `<{}>` on `{}` — {} occurrence(s)\n",
                md_escape(&field.field),
                md_escape(&field.type_name),
                field.occurrences
            ));
        }
        out.push('\n');
    }

    if !report.unused_schema_types.is_empty() {
        out.push_str(&format!(
            "## Known schema types not observed ({})\n\n",
            report.unused_schema_types.len()
        ));
        out.push_str(
            &report
                .unused_schema_types
                .iter()
                .map(|ty| format!("`{}`", md_escape(ty)))
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str("\n\n");
    }

    if !report.errors.is_empty() {
        out.push_str("## Scan errors\n\n");
        for error in &report.errors {
            out.push_str(&format!("- {}\n", md_escape(error)));
        }
    }

    out
}

/// Heuristic: a type is worth a block schema when it carries connector pins or
/// sits directly on a page. Captions, icon tables, and document sections have
/// neither, which keeps `check --fail` focused on real gaps.
pub fn is_schema_candidate(type_name: &str, stats: &crate::stats::TypeStats) -> bool {
    if is_structural_type(type_name) {
        return false;
    }
    !stats.pins.is_empty() || stats.parent_types.contains_key("Page")
}

fn render_type_table(out: &mut String, items: &[&TypeCount]) {
    out.push_str("| Type | Occurrences | Contexts |\n");
    out.push_str("| :--- | ---: | :--- |\n");
    for item in items {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            md_escape(&item.type_name),
            item.occurrences,
            context_summary(&item.root_sections)
        ));
    }
}

fn unknown_field(
    type_name: &str,
    field: &str,
    occurrences: usize,
    type_occurrences: usize,
    kind: &str,
    examples: &[String],
    universal: bool,
) -> UnknownField {
    UnknownField {
        type_name: type_name.to_string(),
        field: field.to_string(),
        occurrences,
        type_occurrences,
        fill_percent: percent(occurrences, type_occurrences),
        inferred_kind: Some(kind.to_string()),
        examples: examples.to_vec(),
        universal,
    }
}

fn compare_unknown(a: &UnknownField, b: &UnknownField) -> std::cmp::Ordering {
    b.occurrences
        .cmp(&a.occurrences)
        .then_with(|| a.type_name.cmp(&b.type_name))
        .then_with(|| a.field.cmp(&b.field))
}

fn child_config_matches(schema: &BlockSchema, tag: &str) -> bool {
    schema.child_configs.iter().any(|kind| match kind {
        ChildConfigKind::Hcl => tag.eq_ignore_ascii_case("HCL"),
        ChildConfigKind::Psd => tag.eq_ignore_ascii_case("PSD"),
        ChildConfigKind::LoggerMailer => {
            matches!(tag, "Logger" | "Mailer" | "LoggerMailer")
        }
        ChildConfigKind::IoData => tag == "IoData",
        ChildConfigKind::CoHist => tag.eq_ignore_ascii_case("COHist"),
        ChildConfigKind::LightScenes => tag.to_ascii_lowercase().contains("lightscene"),
        ChildConfigKind::StatsGroup => tag.eq_ignore_ascii_case("statGrp"),
        ChildConfigKind::Load => tag.eq_ignore_ascii_case("Load"),
        ChildConfigKind::Modes => tag.eq_ignore_ascii_case("Modes"),
        ChildConfigKind::Display => tag.eq_ignore_ascii_case("Display"),
        ChildConfigKind::Entry => tag.eq_ignore_ascii_case("Entry"),
        ChildConfigKind::StateTexts => tag.eq_ignore_ascii_case("StateTexts"),
        ChildConfigKind::Hp => tag.eq_ignore_ascii_case("HP"),
        ChildConfigKind::Ph => tag.eq_ignore_ascii_case("PH"),
        ChildConfigKind::Buttons => tag.eq_ignore_ascii_case("btns"),
    })
}

fn example_values(stats: &AttrStats, limit: usize) -> String {
    if stats.count == 0 {
        return "—".to_string();
    }
    let mut values: Vec<String> = stats
        .examples
        .iter()
        .take(limit)
        .map(|v| format!("`{}`", md_escape(empty_or(v, "<empty>"))))
        .collect();
    if stats.distinct > values.len() {
        values.push(format!("… {} distinct", stats.distinct));
    }
    values.join(", ")
}

fn context_summary(contexts: &BTreeMap<String, usize>) -> String {
    if contexts.is_empty() {
        return "—".to_string();
    }
    contexts
        .iter()
        .map(|(name, count)| format!("`{}` ({})", md_escape(name), count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 * 100.0 / total as f64
    }
}

fn empty_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

fn md_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\r', " ")
        .replace('\n', " ")
        .replace('`', "'")
}
