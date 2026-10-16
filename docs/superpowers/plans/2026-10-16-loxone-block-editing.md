# Loxone Config–Style Block & Periphery Attribute Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable Loxone Config–style block and periphery attribute/parameter editing in `loxone-portable`, backed by official Loxone Knowledge Base documentation (`loxone.com/enen/kb/`) for 100% of the 41 page block types and 90 periphery/system object types in `Klimczaki.Loxone`, with lossless XML serialization and dirty-state tracking.

**Architecture:** Add a zero-GUI Loxone KB schema registry (`crates/loxone-core/src/block_schema.rs`) and lossless XML mutator/serializer (`crates/loxone-core/src/writer.rs`) to `loxone-core`, then upgrade the right-hand Property Inspector (`crates/loxone-gui/src/inspector.rs`) and application shell (`crates/loxone-gui/src/app.rs`, `crates/loxone-gui/src/top_bar.rs`) to render structured, KB-documented property editors that emit `ProjectEdit` commands and persist changes back to `.Loxone` XML files.

**Tech Stack:** Rust 2021, `quick-xml` 0.37, `indexmap` 2.7, `eframe` / `egui` 0.31.

---

## File Structure

- **Create `crates/loxone-core/src/block_schema.rs`**:
  - Defines `BlockSchema`, `AttributeSchema`, `PinParamSchema`, `PinDoc`, `ValueKind`, `ChildConfigKind`, and `lookup_block_schema(type_name: &str) -> Option<&'static BlockSchema>`.
  - Encodes official Loxone Knowledge Base URLs, Polish + English names, summaries, `<C>` attribute schemas, `<Co K="..." Def="..."/>` parameter pin schemas (with units, ranges, enum options, and descriptions), input/output pin documentation, and child configuration tag schemas (`<IoData>`, `<Display>`, `<LoggerMailer>`, `<HCL>`, `<PSD>`, `<COHist>`) for all 41 page block types and all 90 periphery/system object types in `Klimczaki.Loxone`.
- **Create `crates/loxone-core/src/writer.rs`**:
  - Defines `ProjectEdit` (`SetControlAttr`, `SetPinDefault`, `SetChildTagAttr`), `apply_edit(project: &mut LoxoneProject, edit: &ProjectEdit) -> Result<(), LoxoneError>`, `serialize_loxone_xml(project: &LoxoneProject) -> Result<String, LoxoneError>`, and `save_loxone_file(path: &Path, project: &mut LoxoneProject) -> Result<(), LoxoneError>`.
- **Modify `crates/loxone-core/src/model.rs`**:
  - Add `RawNode::set_attr(&mut self, name: &str, value: Option<&str>)` helper to update or remove XML attributes in place while preserving attribute order.
- **Modify `crates/loxone-core/src/parser.rs`**:
  - Make `parse_raw_dom` `pub(crate)` so `writer.rs` can parse `project.raw_xml` into `Vec<RawNode>`, synchronize in-memory `PageBlock` and `TreeNode` mutations onto matching `<C U="...">` elements, and serialize back with UTF-8 BOM (`\u{FEFF}`), tab indentation, and CRLF (`\r\n`) line endings.
- **Modify `crates/loxone-core/src/lib.rs`**:
  - Export `pub mod block_schema;` and `pub mod writer;`.
- **Modify `crates/loxone-gui/src/inspector.rs`**:
  - Replace read-only raw key-value tables with a Loxone Config–style structured Property Inspector returning `InspectorOutput { nav_action: Option<InspectorAction>, edits: Vec<ProjectEdit> }`.
  - Render: (1) KB Documentation Header + `📖 Loxone Docs` button, (2) General & Visualization section (`Title`, `Desc`, Room `Pr` dropdown, Category `Cr` dropdown, `Visu`, `VisuPwd`, `UseInFav`, `Rating`, `Cl` color picker, `<Display>`, `Text`), (3) Block Parameters & Operating Modes (`<C>` attributes + `<Co>` parameter pins with units, ranges, enums, and tooltips), (4) Child Config Tags (`<LoggerMailer>`, `<HCL>`, `<PSD>`), (5) Inline **Linked Target Properties** editor when a page reference block (`InputRef`, `OutputRef`, `OutputRefLM`) links to a `TreeNode` or `PageBlock`, (6) KB-documented Inputs & Outputs with cross-page wire navigation, and (7) Collapsible Advanced / Raw XML Attributes editor.
- **Modify `crates/loxone-gui/src/top_bar.rs` & `crates/loxone-gui/src/app.rs`**:
  - Add `pub is_dirty: bool` to `LoxonePortableApp`, display `*` unsaved indicator in the top bar breadcrumb and highlight the `Save .Loxone` button when `is_dirty == true`, apply `InspectorOutput.edits` via `loxone_core::writer::apply_edit`, and save directly to `loaded_path` (or prompt `FileDialog` if `None`) on `Ctrl+S` / `SaveFile`.

---

### Task 1: Official Loxone KB Schema Registry (`crates/loxone-core/src/block_schema.rs`)

**Files:**
- Create: `crates/loxone-core/src/block_schema.rs`
- Modify: `crates/loxone-core/src/lib.rs`

- [ ] **Step 1: Write the failing unit tests in `crates/loxone-core/src/block_schema.rs` and export module in `crates/loxone-core/src/lib.rs`**

Add `pub mod block_schema;` to `crates/loxone-core/src/lib.rs` and create `crates/loxone-core/src/block_schema.rs` with the schema types and unit tests verifying:
1. `lookup_block_schema` resolves representative complex blocks (`LightController2`, `AutoJalousie`, `HeatIRoomController2`, `ClimateControllerUS`, `HVACController`, `MusicPlayer`, `Alarm`, `SmokeAlarm`, `PresenceDetector`, `MeterAbsBi`, `MultiClick`, `OutputRefLM`, `TreeAsensor`, `VirtualOutCmd`) with valid `https://www.loxone.com/enen/kb/` URLs, attribute schemas, pin parameter schemas, and pin docs.
2. Every single `<C Type="...">` across all 28 pages (532 page blocks, 41 unique types) and all periphery/system tree nodes (961 nodes, 90 unique types) in `Klimczaki.Loxone` resolves to `Some(schema)` via `lookup_block_schema`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TreeNode;
    use crate::parser::load_loxone_file;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn collect_tree_types(node: &TreeNode, out: &mut BTreeSet<String>) {
        out.insert(node.control_type.clone());
        for child in &node.children {
            collect_tree_types(child, out);
        }
    }

    #[test]
    fn test_klimczaki_100_percent_schema_coverage() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        let mut page_types = BTreeSet::new();
        for page in &project.pages {
            for block in &page.blocks {
                page_types.insert(block.block_type.clone());
            }
        }
        assert_eq!(page_types.len(), 41);

        for ty in &page_types {
            let schema = lookup_block_schema(ty)
                .unwrap_or_else(|| panic!("Missing BlockSchema for page block type: {ty}"));
            assert_eq!(schema.type_name, ty);
            assert!(
                schema.kb_url.starts_with("https://www.loxone.com/enen/kb/"),
                "Invalid kb_url for {ty}: {}",
                schema.kb_url
            );
            assert!(!schema.summary.is_empty(), "Empty summary for {ty}");
        }

        let mut tree_types = BTreeSet::new();
        for node in &project.periphery_tree {
            collect_tree_types(node, &mut tree_types);
        }
        for node in &project.system_tree {
            collect_tree_types(node, &mut tree_types);
        }

        for ty in &tree_types {
            let schema = lookup_block_schema(ty)
                .unwrap_or_else(|| panic!("Missing BlockSchema for tree node type: {ty}"));
            assert_eq!(schema.type_name, ty);
            assert!(
                schema.kb_url.starts_with("https://www.loxone.com/enen/kb/"),
                "Invalid kb_url for {ty}: {}",
                schema.kb_url
            );
        }
    }

    #[test]
    fn test_kb_parameter_and_attribute_metadata_details() {
        let lc2 = lookup_block_schema("LightController2").unwrap();
        assert_eq!(lc2.display_name_en, "Lighting Controller");
        assert!(lc2.pin_params.iter().any(|p| p.pin_key == "Step" && p.unit == Some("%")));
        assert!(lc2.pin_params.iter().any(|p| p.pin_key == "Mt"));
        assert!(lc2.child_configs.contains(&ChildConfigKind::Hcl));

        let jal = lookup_block_schema("AutoJalousie").unwrap();
        assert!(jal.pin_params.iter().any(|p| p.pin_key == "Tu" && p.unit == Some("s")));
        assert!(jal.pin_params.iter().any(|p| p.pin_key == "Td" && p.unit == Some("s")));
        assert!(jal.child_configs.contains(&ChildConfigKind::Psd));

        let irc = lookup_block_schema("HeatIRoomController2").unwrap();
        assert!(irc.attributes.iter().any(|a| a.xml_attr == "UseH" && a.unit == Some("°C")));
        assert!(irc.pin_params.iter().any(|p| p.pin_key == "Tc" && p.unit == Some("°C")));

        let lm = lookup_block_schema("OutputRefLM").unwrap();
        assert!(lm.child_configs.contains(&ChildConfigKind::LoggerMailer));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-core block_schema`
Expected: FAIL (missing implementation of `lookup_block_schema` and schema definitions).

- [ ] **Step 3: Implement `crates/loxone-core/src/block_schema.rs` with full KB documentation for all 41 page block types and 90 periphery/system types**

Use the ground-truth Loxone KB documentation and XML attributes/pins extracted in `<appDataDir>/brain/6757a976-a0c0-41d5-90c1-7661e2096e02/scratch/` (`loxone_kb_Part1.txt`, `loxone_kb_Part2.txt`, `page_c_types.txt`, `non_page_c_types.txt`) to populate `BlockSchema`, `AttributeSchema`, `PinParamSchema`, `PinDoc`, `ValueKind`, and `ChildConfigKind` for every type. Include helper methods on `BlockSchema`:
- `find_attr(&self, xml_attr: &str) -> Option<&AttributeSchema>`
- `find_pin_param(&self, pin_key: &str) -> Option<&PinParamSchema>`
- `find_pin_doc(&self, pin_key: &str) -> Option<&PinDoc>` (supporting indexed prefix lookup like `I1`..`I8`, `AI1`..`AI8`, `AQ1`..`AQ16`, `Q1`..`Q16`, `Dis1`..`Dis6`, `O1`..`O8`, `M1`..`M4`, `hc1`..`hc12`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-core block_schema`
Expected: PASS with 0 warnings.

- [ ] **Step 5: Commit Task 1**

```bash
git add crates/loxone-core/src/lib.rs crates/loxone-core/src/block_schema.rs
git commit -m "feat(core): add Loxone KB schema registry for all project block and periphery types"
```

---

### Task 2: In-Memory Project Mutator & Lossless XML Serializer (`crates/loxone-core/src/writer.rs`)

**Files:**
- Modify: `crates/loxone-core/src/model.rs`
- Modify: `crates/loxone-core/src/parser.rs`
- Create: `crates/loxone-core/src/writer.rs`
- Modify: `crates/loxone-core/src/lib.rs`

- [ ] **Step 1: Write the failing unit tests in `crates/loxone-core/src/writer.rs`**

Add `pub mod writer;` to `crates/loxone-core/src/lib.rs` and create `crates/loxone-core/src/writer.rs` with tests verifying:
1. `apply_edit` with `ProjectEdit::SetControlAttr` updates `title`, `color_rgb`, `text_content`, `ref_uuid`, and `raw_attributes` on both a `PageBlock` and a `TreeNode`.
2. `apply_edit` with `ProjectEdit::SetPinDefault` updates `ConnectorPin.default_value` on a `PageBlock` or `TreeNode` (and creates/updates `<Co K="..." Def="..."/>`).
3. `apply_edit` with `ProjectEdit::SetChildTagAttr` updates `<IoData Pr="..." Cr="..."/>` (and syncs `room_uuid` / `category_uuid`), `<Display>`, `<LoggerMailer>`, `<HCL>`, and `<PSD>` on `extra_children`, creating the child `RawNode` if it did not previously exist.
4. `serialize_loxone_xml` after applying edits to `Klimczaki.Loxone` produces valid XML starting with `\u{FEFF}<?xml version="1.0" encoding="utf-8"?>\r\n`, and re-parsing that XML via `parse_loxone_xml` preserves all 28 pages, 16 rooms, 33 categories, all wires, and reflects the edited `<C>` attributes, `<Co Def="..."/>` values, and child tag attributes.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{load_loxone_file, parse_loxone_xml};
    use std::path::PathBuf;

    #[test]
    fn test_apply_edits_and_roundtrip_serialize_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");

        let target_block_uuid = project.pages[0].blocks[0].uuid.clone();
        let target_pin_key = project.pages[0].blocks[0].pins[0].key.clone();
        let first_room_uuid = project.places.keys().next().unwrap().clone();
        let first_cat_uuid = project.categories.keys().next().unwrap().clone();

        // 1. Edit block Title and Color
        apply_edit(
            &mut project,
            &ProjectEdit::SetControlAttr {
                control_uuid: target_block_uuid.clone(),
                attr_name: "Title".to_string(),
                value: Some("Edited Block Title".to_string()),
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::SetControlAttr {
                control_uuid: target_block_uuid.clone(),
                attr_name: "Cl".to_string(),
                value: Some("12,34,56".to_string()),
            },
        )
        .unwrap();

        assert_eq!(project.pages[0].blocks[0].title, "Edited Block Title");
        assert_eq!(project.pages[0].blocks[0].color_rgb, Some([12, 34, 56]));

        // 2. Edit Pin Default value
        apply_edit(
            &mut project,
            &ProjectEdit::SetPinDefault {
                control_uuid: target_block_uuid.clone(),
                pin_key: target_pin_key.clone(),
                default_value: Some("42.5".to_string()),
            },
        )
        .unwrap();
        assert_eq!(
            project.pages[0].blocks[0].pins[0].default_value.as_deref(),
            Some("42.5")
        );

        // 3. Edit Child Tag <IoData Pr="..." Cr="..."/>
        apply_edit(
            &mut project,
            &ProjectEdit::SetChildTagAttr {
                control_uuid: target_block_uuid.clone(),
                child_tag: "IoData".to_string(),
                attr_name: "Pr".to_string(),
                value: Some(first_room_uuid.clone()),
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::SetChildTagAttr {
                control_uuid: target_block_uuid.clone(),
                child_tag: "IoData".to_string(),
                attr_name: "Cr".to_string(),
                value: Some(first_cat_uuid.clone()),
            },
        )
        .unwrap();
        assert_eq!(
            project.pages[0].blocks[0].room_uuid.as_deref(),
            Some(first_room_uuid.as_str())
        );
        assert_eq!(
            project.pages[0].blocks[0].category_uuid.as_deref(),
            Some(first_cat_uuid.as_str())
        );

        // 4. Serialize and re-parse
        let xml = serialize_loxone_xml(&project).expect("should serialize XML");
        assert!(xml.starts_with("\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n"));

        let reparsed = parse_loxone_xml(&xml).expect("reparsed XML should be valid");
        assert_eq!(reparsed.pages.len(), 28);
        assert_eq!(reparsed.places.len(), 16);
        assert_eq!(reparsed.categories.len(), 33);

        let blk = &reparsed.pages[0].blocks[0];
        assert_eq!(blk.uuid, target_block_uuid);
        assert_eq!(blk.title, "Edited Block Title");
        assert_eq!(blk.color_rgb, Some([12, 34, 56]));
        assert_eq!(blk.pins[0].default_value.as_deref(), Some("42.5"));
        assert_eq!(blk.room_uuid.as_deref(), Some(first_room_uuid.as_str()));
        assert_eq!(blk.category_uuid.as_deref(), Some(first_cat_uuid.as_str()));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-core writer`
Expected: FAIL (missing `writer` implementation).

- [ ] **Step 3: Implement `RawNode::set_attr` in `model.rs`, expose `parse_raw_dom` in `parser.rs`, and implement `writer.rs`**

1. In `crates/loxone-core/src/model.rs`, add `RawNode::set_attr(&mut self, name: &str, value: Option<&str>)` to update an existing `(k, v)` pair in `self.attributes`, append if missing when `Some`, or remove if `None`.
2. In `crates/loxone-core/src/parser.rs`, change `fn parse_raw_dom` to `pub(crate) fn parse_raw_dom`.
3. In `crates/loxone-core/src/writer.rs`, implement:
   - `ProjectEdit` enum (`SetControlAttr`, `SetPinDefault`, `SetChildTagAttr`).
   - `apply_edit(project: &mut LoxoneProject, edit: &ProjectEdit) -> Result<(), LoxoneError>` which mutates the matching `PageBlock` or `TreeNode` (or `Category` / `Place` when selected) and updates derived fields (`title`, `color_rgb`, `text_content`, `ref_uuid`, `room_uuid`, `category_uuid`, `pins[i].default_value`, `extra_children`, `xref` if title changed).
   - `serialize_loxone_xml(project: &LoxoneProject) -> Result<String, LoxoneError>` which parses `project.raw_xml` via `parse_raw_dom`, synchronizes all `PageBlock`, `TreeNode`, `Category`, and `Place` attributes, `<Co>` `Def` attributes, and `extra_children` into the `RawNode` tree, and serializes with `\u{FEFF}<?xml version="1.0" encoding="utf-8"?>\r\n`, tab indentation, and proper XML attribute/text escaping (`&`, `<`, `>`, `"`, `\r`, `\n`).
   - `save_loxone_file(path: &Path, project: &mut LoxoneProject) -> Result<(), LoxoneError>` which serializes `project`, updates `project.raw_xml = Some(xml.clone())`, and writes `xml` to `path`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-core`
Expected: PASS with 0 warnings.

- [ ] **Step 5: Commit Task 2**

```bash
git add crates/loxone-core/src/model.rs crates/loxone-core/src/parser.rs crates/loxone-core/src/writer.rs crates/loxone-core/src/lib.rs
git commit -m "feat(core): add ProjectEdit mutator and lossless .Loxone XML serializer"
```

---

### Task 3: Loxone Config–Style Property Inspector (`crates/loxone-gui/src/inspector.rs`)

**Files:**
- Modify: `crates/loxone-gui/src/inspector.rs`

- [ ] **Step 1: Write the failing unit tests in `crates/loxone-gui/src/inspector.rs`**

Add unit tests in `crates/loxone-gui/src/inspector.rs` testing:
1. `InspectorOutput` returned by `render_right_inspector`.
2. Headless `egui` rendering of `render_right_inspector` across:
   - Complex controller blocks (`LightController2`, `AutoJalousie`, `HeatIRoomController2`, `OutputRefLM`)
   - Reference blocks (`InputRef`, `OutputRef`) with inline Linked Target Properties
   - Periphery `TreeNode` (`TreeAsensor`, `VirtualOutCmd`)
   - Every single block on Page 0..27 in `Klimczaki.Loxone` without panicking.

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-gui inspector`
Expected: FAIL (`InspectorOutput` not yet implemented).

- [ ] **Step 3: Implement the Loxone Config–style Property Inspector in `crates/loxone-gui/src/inspector.rs`**

Update `render_right_inspector(ui: &mut Ui, project: &LoxoneProject, selected_uuid: Option<&str>) -> InspectorOutput` to render:
1. **Header & Official Loxone KB Documentation Card**:
   - Block/object title, `Type` badge, Polish & English display names (`schema.display_name_pl`, `schema.display_name_en`), concise KB summary (`schema.summary`), and a `📖 Loxone Docs` button that calls `ui.ctx().open_url(egui::OpenUrl::new_tab(schema.kb_url))`.
2. **General & Visualization Section**:
   - Editable `Title` (`TextEdit::singleline`) and `Desc` (`TextEdit::singleline`).
   - Room (`<IoData Pr="..."/>`) `ComboBox` populated from `project.places` (plus `"— Unassigned —"`).
   - Category (`<IoData Cr="..."/>`) `ComboBox` populated from `project.categories` (plus `"— Unassigned —"`).
   - Visualization controls: `Visu` checkbox, `UseInFav` checkbox, `VisuPwd` selector, `Rating` (`0..=5`), block color picker (`Cl` `R,G,B` with `ui.color_edit_button_srgb`), `<Display Unit="..." Format="..."/>` editor when supported, and multiline `Text` editor for `Text` blocks.
3. **Block Parameters & Operating Modes Section**:
   - Schema-driven `<C>` attributes (`schema.attributes`) with proper widget per `ValueKind` (`Bool`, `Int`, `Float`, `Seconds`, `Percent`, `Degrees`, `Temperature`, `Enum`, `Text`, `MultilineText`, `RgbColor`, `UuidRef`), unit suffix, and KB tooltip.
   - Schema-driven `<Co K="..." Def="..."/>` parameter pins (`schema.pin_params`) with proper widget, unit suffix, and KB tooltip. If a pin exists in `schema.pin_params` or `block.pins` (with `PinKind::Parameter` or `default_value.is_some()`), editing emits `ProjectEdit::SetPinDefault`.
4. **Child Configuration Sections (`<LoggerMailer>`, `<HCL>`, `<PSD>`)**:
   - When `schema.child_configs` or `extra_children` contains `LoggerMailer`, `Hcl`, or `Psd`, render structured editors for `<LoggerMailer Type="..." Address="..." Subject="..." TextON="..." TextOFF="..." RefCaller="..." RefNotification="..."/>`, `<HCL Mode="..." Min="..." Max="..." Offset="..."/>`, and `<PSD Type="..." Az="..." El="..."/>` emitting `ProjectEdit::SetChildTagAttr`.
5. **Inline Linked Target Properties Section (for `InputRef`, `OutputRef`, `OutputRefLM`)**:
   - When a `PageBlock` has `ref_uuid` resolving to a `TreeNode` (or another `PageBlock`), render both the `"Select Target in Tree"` button AND an inline collapsible **"Linked Target Properties"** editor showing the target's KB summary, `Title`, Room/Category, and `<C>` attributes (e.g. `MinVal`, `MaxVal`, `MinMV`, `MaxMV`, `Unit`, `Corr`, `CmdOn`, `CmdOff`, `HttpMethod`, `Text`, `Frequency`, `Volume`, `ConfirmText`, `I/O flags`) emitting `ProjectEdit` targeting the linked `ref_uuid` directly!
6. **KB-Documented Inputs & Outputs + Cross-Page Navigation**:
   - Render inputs and outputs enriched with `schema.find_pin_doc(&pin.key)` descriptions and clickable cross-page wire navigation links.
7. **Collapsible Advanced / Raw XML Attributes Editor**:
   - Collapsible section showing all `raw_attributes` (except immutable `U`, `Type`, `V`) with inline `TextEdit::singleline` editors and an option to add/edit custom attributes so 100% of XML attributes are always editable.

- [ ] **Step 4: Wire `InspectorOutput` in `crates/loxone-gui/src/app.rs` so `cargo test -p loxone-gui` compiles and passes**

Update the call sites of `render_right_inspector` in `crates/loxone-gui/src/app.rs` to handle `InspectorOutput`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-gui inspector`
Expected: PASS with 0 warnings.

- [ ] **Step 6: Commit Task 3**

```bash
git add crates/loxone-gui/src/inspector.rs crates/loxone-gui/src/app.rs
git commit -m "feat(gui): implement Loxone Config-style KB-backed property inspector"
```

---

### Task 4: App Shell Integration, Unsaved Indicator & Save Workflow (`crates/loxone-gui/src/app.rs` & `crates/loxone-gui/src/top_bar.rs`)

**Files:**
- Modify: `crates/loxone-gui/src/top_bar.rs`
- Modify: `crates/loxone-gui/src/app.rs`

- [ ] **Step 1: Write the failing unit tests in `crates/loxone-gui/src/app.rs` and `crates/loxone-gui/src/top_bar.rs`**

Add unit tests verifying:
1. `build_file_toolbar_items(can_save, is_dirty)` marks the Save button with `accent: true` and `"Save .Loxone *"` when `is_dirty == true`.
2. Applying `ProjectEdit` on `LoxonePortableApp` sets `app.is_dirty = true`, updates the project in memory, and calling `app.save_xml_to_path(&temp_file)` writes the updated XML to disk, resets `app.is_dirty = false`, and re-loading `&temp_file` into a fresh `LoxonePortableApp` preserves all edited attributes and pin defaults.

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-gui`
Expected: FAIL (`is_dirty` and `apply_project_edit` not yet wired).

- [ ] **Step 3: Implement `is_dirty` indicator in `top_bar.rs` and edit/save handling in `app.rs`**

1. In `crates/loxone-gui/src/top_bar.rs`:
   - Update `build_file_toolbar_items(can_save: bool, is_dirty: bool) -> Vec<ToolbarItem>` so that when `is_dirty` is `true`, the Save button label is `"Save .Loxone *"` and `accent: true`.
   - Update `render_top_bar` to accept `is_dirty: bool` and prepend/append `*` to the project title breadcrumb when `is_dirty == true`.
2. In `crates/loxone-gui/src/app.rs`:
   - Add `pub is_dirty: bool` to `LoxonePortableApp`.
   - Reset `self.is_dirty = false` in `load_file` and `load_downloaded_xml`.
   - Add `pub fn apply_project_edit(&mut self, edit: loxone_core::writer::ProjectEdit)` which applies the edit via `loxone_core::writer::apply_edit` and sets `self.is_dirty = true`.
   - In `save_xml_to_path(&mut self, path: &Path)`: if `self.is_dirty` is `true`, call `loxone_core::writer::save_loxone_file(path, proj)` and set `self.is_dirty = false`; otherwise write `proj.raw_xml` directly and set `self.is_dirty = false`.
   - In `PaletteAction::SaveFile`: if `self.loaded_path` is `Some(existing_path)`, save directly to `existing_path` (or if shift is held / `loaded_path` is `None`, open the `rfd::FileDialog`).

- [ ] **Step 4: Run full workspace tests to verify everything passes with zero warnings**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test --workspace`
Expected: PASS across both `loxone-core` and `loxone-gui` with 0 warnings.

- [ ] **Step 5: Commit Task 4**

```bash
git add crates/loxone-gui/src/top_bar.rs crates/loxone-gui/src/app.rs
git commit -m "feat(gui): wire inspector edits, dirty state indicator, and lossless XML save"
```
