//! Exports: a schematic page as SVG, and a printable HTML project documentation.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::model::{LoxoneProject, Page, PageBlock, PinKind};
use crate::project_admin::{
    calendar_entries, find_node_by_uuid, find_nodes_by_type, group_member_uuids,
    mode_title_for_number, operating_modes, recurrence_label,
};

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Display label of a block: its title, or for reference bars the title of
/// the referenced periphery/system object.
pub fn block_label(project: &LoxoneProject, b: &PageBlock) -> String {
    if !b.title.trim().is_empty() {
        return b.title.clone();
    }
    if let Some(r) = &b.ref_uuid {
        if let Some(n) = find_node_by_uuid(project, r) {
            return n.title.clone();
        }
    }
    b.block_type.clone()
}

/// Loxone page coordinates are scaled down by this factor for the SVG.
const SCALE: f32 = 0.1;

/// Renders a page as a standalone SVG drawing (blocks, pin names, wires).
pub fn page_to_svg(project: &LoxoneProject, page: &Page) -> String {
    let placed: Vec<&PageBlock> = page.blocks.iter().filter(|b| b.has_position).collect();
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for b in &placed {
        let (x1, x2) = (b.px.min(b.px2), b.px.max(b.px2));
        let (y1, y2) = (b.py.min(b.py2), b.py.max(b.py2));
        minx = minx.min(x1);
        miny = miny.min(y1);
        maxx = maxx.max(x2.max(x1 + 400.0));
        maxy = maxy.max(y2.max(y1 + 200.0));
    }
    if placed.is_empty() {
        minx = 0.0;
        miny = 0.0;
        maxx = 4000.0;
        maxy = 2000.0;
    }
    let margin = 40.0;
    let w = (maxx - minx) * SCALE + margin * 2.0;
    let h = (maxy - miny) * SCALE + margin * 2.0 + 30.0;
    let tx = |x: f32| (x - minx) * SCALE + margin;
    let ty = |y: f32| (y - miny) * SCALE + margin + 30.0;

    // Pin positions: (pin uuid) -> (x, y)
    let mut pin_pos: HashMap<&str, (f32, f32)> = HashMap::new();
    let mut body = String::new();
    for b in &placed {
        let x1 = tx(b.px.min(b.px2));
        let y1 = ty(b.py.min(b.py2));
        let mut bw = ((b.px2 - b.px).abs()) * SCALE;
        let inputs: Vec<_> = b.pins.iter().filter(|p| p.pin_kind != PinKind::Output).collect();
        let outputs: Vec<_> = b.pins.iter().filter(|p| p.pin_kind == PinKind::Output).collect();
        let rows = inputs.len().max(outputs.len()).max(1) as f32;
        let mut bh = ((b.py2 - b.py).abs()) * SCALE;
        if bw < 40.0 {
            bw = if b.is_ref_bar() { 120.0 } else { 90.0 };
        }
        bh = bh.max(18.0 + rows * 12.0);
        let [r, g, bl] = b.color_rgb.unwrap_or(if b.is_ref_bar() {
            [0x6F, 0xB3, 0x44]
        } else {
            [0x3A, 0x45, 0x52]
        });
        let label = xml_escape(&block_label(project, b));
        if b.is_note() {
            let text = xml_escape(b.text_content.as_deref().unwrap_or(&b.title));
            let _ = writeln!(
                body,
                r##"<g><rect x="{x1:.1}" y="{y1:.1}" width="{bw:.1}" height="{bh:.1}" fill="#FFF7C2" stroke="#C9B458"/><text x="{:.1}" y="{:.1}" font-size="9" fill="#333">{text}</text></g>"##,
                x1 + 4.0,
                y1 + 12.0
            );
            continue;
        }
        let _ = writeln!(
            body,
            r##"<g><rect x="{x1:.1}" y="{y1:.1}" width="{bw:.1}" height="{bh:.1}" rx="3" fill="rgb({r},{g},{bl})" stroke="#1E252C"/><text x="{:.1}" y="{:.1}" font-size="9" font-weight="bold" fill="#FFFFFF">{label}</text>"##,
            x1 + 4.0,
            y1 + 11.0
        );
        for (i, p) in inputs.iter().enumerate() {
            let py = y1 + 22.0 + i as f32 * 12.0;
            pin_pos.insert(p.uuid.as_str(), (x1, py));
            let _ = writeln!(
                body,
                r##"<circle cx="{x1:.1}" cy="{py:.1}" r="2" fill="#9CCB6A"/><text x="{:.1}" y="{:.1}" font-size="7" fill="#DDE3EA">{}</text>"##,
                x1 + 4.0,
                py + 2.5,
                xml_escape(&p.key)
            );
        }
        for (i, p) in outputs.iter().enumerate() {
            let py = y1 + 22.0 + i as f32 * 12.0;
            let x2 = x1 + bw;
            pin_pos.insert(p.uuid.as_str(), (x2, py));
            let _ = writeln!(
                body,
                r##"<circle cx="{x2:.1}" cy="{py:.1}" r="2" fill="#F2B84B"/><text x="{:.1}" y="{:.1}" font-size="7" text-anchor="end" fill="#DDE3EA">{}</text>"##,
                x2 - 4.0,
                py + 2.5,
                xml_escape(&p.key)
            );
        }
        body.push_str("</g>\n");
    }
    let mut wires = String::new();
    for b in &placed {
        for p in &b.pins {
            let Some(&(dx, dy)) = pin_pos.get(p.uuid.as_str()) else { continue };
            for l in &p.incoming_links {
                if l.is_hidden() {
                    continue;
                }
                if let Some(&(sx, sy)) = pin_pos.get(l.source_pin_uuid.as_str()) {
                    let mx = (sx + dx) / 2.0;
                    let _ = writeln!(
                        wires,
                        r##"<path d="M{sx:.1},{sy:.1} L{mx:.1},{sy:.1} L{mx:.1},{dy:.1} L{dx:.1},{dy:.1}" fill="none" stroke="#69C350" stroke-width="1"/>"##
                    );
                }
            }
        }
    }
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}" viewBox="0 0 {w:.0} {h:.0}" font-family="Segoe UI, Arial, sans-serif">
<rect width="100%" height="100%" fill="#F4F6F8"/>
<text x="{margin}" y="28" font-size="16" font-weight="bold" fill="#1E252C">{}</text>
{wires}{body}</svg>
"##,
        xml_escape(&page.title)
    )
}

/// Builds a self-contained, printable HTML documentation of the project.
pub fn project_documentation_html(project: &LoxoneProject) -> String {
    let m = &project.metadata;
    let mut s = String::new();
    let e = xml_escape;
    let _ = write!(
        s,
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>{t} – Documentation</title>
<style>body{{font-family:Segoe UI,Arial,sans-serif;margin:32px;color:#222}}h1{{color:#2f7d1f}}h2{{border-bottom:2px solid #69C350;padding-bottom:4px;margin-top:32px}}
table{{border-collapse:collapse;width:100%;margin:8px 0 16px}}td,th{{border:1px solid #ccc;padding:4px 8px;text-align:left;font-size:13px}}th{{background:#eef5ea}}
@media print{{h2{{page-break-before:auto}} .page{{page-break-inside:avoid}}}}</style></head><body>
<h1>{t}</h1>"#,
        t = e(&m.title)
    );
    s.push_str("<h2>Project properties</h2><table>");
    for (label, v) in [
        ("Created by", &m.creator),
        ("Street", &m.street),
        ("Post code", &m.postcode),
        ("Town", &m.town),
        ("Country", &m.country),
        ("Latitude", &m.latitude),
        ("Longitude", &m.longitude),
        ("Currency", &m.currency),
    ] {
        let _ = write!(s, "<tr><th>{}</th><td>{}</td></tr>", label, e(v));
    }
    s.push_str("</table>");

    s.push_str("<h2>Rooms</h2><table><tr><th>Room</th><th>Area (m²)</th></tr>");
    for p in project.places.values() {
        let _ = write!(
            s,
            "<tr><td>{}</td><td>{}</td></tr>",
            e(&p.title),
            e(p.sqm.as_deref().unwrap_or(""))
        );
    }
    s.push_str("</table><h2>Categories</h2><table><tr><th>Category</th></tr>");
    for c in project.categories.values() {
        let _ = write!(s, "<tr><td>{}</td></tr>", e(&c.title));
    }
    s.push_str("</table>");

    s.push_str("<h2>Users</h2><table><tr><th>User</th><th>Name</th><th>E-mail</th><th>Groups</th></tr>");
    let groups = find_nodes_by_type(project, "UserGroup");
    for u in find_nodes_by_type(project, "User") {
        let gs: Vec<String> = groups
            .iter()
            .filter(|g| group_member_uuids(g).contains(&u.uuid))
            .map(|g| g.title.clone())
            .collect();
        let _ = write!(
            s,
            "<tr><td>{}</td><td>{} {}</td><td>{}</td><td>{}</td></tr>",
            e(&u.title),
            e(u.attr("FiN").unwrap_or("")),
            e(u.attr("LaN").unwrap_or("")),
            e(u.attr("Ema").unwrap_or("")),
            e(&gs.join(", "))
        );
    }
    s.push_str("</table>");

    s.push_str("<h2>Operating modes</h2><table><tr><th>#</th><th>Mode</th></tr>");
    for md in operating_modes(project) {
        let _ = write!(s, "<tr><td>{}</td><td>{}</td></tr>", md.number, e(&md.title));
    }
    s.push_str("</table><h2>Calendar</h2><table><tr><th>Entry</th><th>Mode</th><th>From</th><th>To</th><th>Recurrence</th></tr>");
    for c in calendar_entries(project) {
        let _ = write!(
            s,
            "<tr><td>{}</td><td>{}</td><td>{:02}.{:02}</td><td>{:02}.{:02}</td><td>{}</td></tr>",
            e(&c.title),
            e(&mode_title_for_number(project, c.mode_number).unwrap_or_else(|| c.mode_number.to_string())),
            c.day,
            c.month,
            c.day2,
            c.month2,
            e(&recurrence_label(c.recurrence))
        );
    }
    s.push_str("</table>");

    s.push_str("<h2>Pages</h2>");
    for page in &project.pages {
        let room = page
            .room_uuid
            .as_ref()
            .and_then(|r| project.places.get(r))
            .map(|p| p.title.clone())
            .unwrap_or_default();
        let _ = write!(
            s,
            "<div class=\"page\"><h3>{}</h3><p>{}{} blocks</p><table><tr><th>Block</th><th>Type</th><th>Connected inputs</th></tr>",
            e(&page.title),
            if room.is_empty() { String::new() } else { format!("Room: {} · ", e(&room)) },
            page.blocks.len()
        );
        for b in page.blocks.iter().filter(|b| !b.is_note()) {
            let connected: Vec<&str> = b
                .pins
                .iter()
                .filter(|p| p.has_visible_incoming_links())
                .map(|p| p.key.as_str())
                .collect();
            let _ = write!(
                s,
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                e(&block_label(project, b)),
                e(&b.block_type),
                e(&connected.join(", "))
            );
        }
        s.push_str("</table></div>");
    }
    s.push_str("</body></html>\n");
    s
}

/// Builds a comprehensive HTML Technical Report covering hardware topology (Link / Tree / Air),
/// serial numbers, room & category counts, schematic complexity, and Project Analysis diagnostics.
pub fn technical_report_html(project: &LoxoneProject) -> String {
    let m = &project.metadata;
    let e = xml_escape;
    let hw_items = crate::periphery_provisioning::collect_provisioned_hardware(project);
    let diags = crate::linter::lint_project(project);

    let total_blocks: usize = project.pages.iter().map(|p| p.blocks.len()).sum();
    let total_wires: usize = project
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .flat_map(|b| b.pins.iter())
        .map(|pin| pin.incoming_links.len())
        .sum();

    let mut s = String::new();
    let _ = write!(
        s,
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>{t} – Technical Report</title>
<style>body{{font-family:Segoe UI,Arial,sans-serif;margin:32px;color:#1e252c}}h1{{color:#2f7d1f;margin-bottom:4px}}h2{{border-bottom:2px solid #69C350;padding-bottom:4px;margin-top:28px}}
.kpi-row{{display:flex;gap:14px;flex-wrap:wrap;margin:14px 0}}.kpi{{border:1px solid #cfd8dc;border-radius:6px;padding:10px 16px;background:#f7faf6;min-width:130px}}
.kpi .val{{font-size:20px;font-weight:bold;color:#2f7d1f}}.kpi .lbl{{font-size:12px;color:#546e7a}}
table{{border-collapse:collapse;width:100%;margin:8px 0 16px}}td,th{{border:1px solid #cfd8dc;padding:5px 8px;text-align:left;font-size:12.5px}}th{{background:#eef5ea}}
</style></head><body>
<h1>{t} — Technical Report</h1>
<p>ConfigVersion: <b>{cv}</b> · Pages: <b>{pg}</b> · Blocks: <b>{bl}</b> · Wires: <b>{wr}</b> · Provisioned Hardware: <b>{hw}</b></p>"#,
        t = e(if m.title.is_empty() { "Loxone Project" } else { &m.title }),
        cv = e(if m.config_version.is_empty() { "17021021" } else { &m.config_version }),
        pg = project.pages.len(),
        bl = total_blocks,
        wr = total_wires,
        hw = hw_items.len(),
    );

    let _ = write!(
        s,
        r#"<div class="kpi-row">
<div class="kpi"><div class="val">{}</div><div class="lbl">Schematic Pages</div></div>
<div class="kpi"><div class="val">{}</div><div class="lbl">Logic Blocks</div></div>
<div class="kpi"><div class="val">{}</div><div class="lbl">Wire Links</div></div>
<div class="kpi"><div class="val">{}</div><div class="lbl">Hardware Devices</div></div>
<div class="kpi"><div class="val">{}</div><div class="lbl">Rooms</div></div>
<div class="kpi"><div class="val">{}</div><div class="lbl">Analysis Findings</div></div>
</div>"#,
        project.pages.len(),
        total_blocks,
        total_wires,
        hw_items.len(),
        project.places.len(),
        diags.len(),
    );

    s.push_str("<h2>Hardware & Bus Inventory (Link / Tree / Air)</h2><table><tr><th>Device</th><th>Type</th><th>Serial Number</th><th>Parent Bus</th><th>I/O Channels</th></tr>");
    for d in &hw_items {
        let sn = if d.serial.trim().is_empty() {
            "Unconfigured"
        } else {
            d.serial.as_str()
        };
        let _ = write!(
            s,
            "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
            e(&d.title),
            e(&d.node_type),
            e(sn),
            e(&d.parent_label),
            d.io_count,
        );
    }
    s.push_str("</table>");

    s.push_str("<h2>Project Analysis Summary</h2><table><tr><th>Category</th><th>Severity</th><th>Code</th><th>Page / Scope</th><th>Object</th><th>Details</th></tr>");
    for d in &diags {
        let sev = match d.severity {
            crate::linter::DiagnosticSeverity::Error => "ERROR",
            crate::linter::DiagnosticSeverity::Warning => "WARN",
            crate::linter::DiagnosticSeverity::Info => "INFO",
        };
        let _ = write!(
            s,
            "<tr><td>{}</td><td><b>{}</b></td><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
            e(d.category().label()),
            sev,
            d.code,
            e(&d.page_title),
            e(&d.block_title),
            e(&d.message),
        );
    }
    s.push_str("</table></body></html>\n");
    s
}

/// Generates a CSV Bill of Materials (BOM) / Component List (`Generate Component List`)
/// covering all hardware devices, aggregated hardware counts by type, and function block usage.
pub fn export_component_bom_csv(project: &LoxoneProject) -> String {
    let hw_items = crate::periphery_provisioning::collect_provisioned_hardware(project);
    let mut out =
        String::from("PartGroup,ComponentType,Title,SerialNumber,BusOrPage,Quantity,IOChannels\n");
    let esc = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));

    let mut hw_counts: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();
    for d in &hw_items {
        let entry = hw_counts.entry(d.node_type.clone()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += d.io_count;
        let _ = writeln!(
            out,
            "Hardware,{},{},{},{},1,{}",
            esc(&d.node_type),
            esc(&d.title),
            esc(&d.serial),
            esc(&d.parent_label),
            d.io_count
        );
    }

    for (node_type, (count, total_io)) in &hw_counts {
        let _ = writeln!(
            out,
            "HardwareSummary,{},{},\"\",\"\",{},{}",
            esc(node_type),
            esc(&format!("Total {node_type}")),
            count,
            total_io
        );
    }

    let mut block_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for page in &project.pages {
        for block in &page.blocks {
            if !block.is_ref_bar() && !block.is_note() {
                *block_counts.entry(block.block_type.clone()).or_insert(0) += 1;
            }
        }
    }
    for (block_type, count) in &block_counts {
        let _ = writeln!(
            out,
            "FunctionBlockSummary,{},{},\"\",\"\",{},0",
            esc(block_type),
            esc(&format!("Total {block_type}")),
            count
        );
    }

    out
}
