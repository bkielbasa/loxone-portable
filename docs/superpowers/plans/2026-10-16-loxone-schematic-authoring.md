# Loxone Portable Phase 3 — Interactive Schematic & Project Authoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform `loxone-portable` into a full interactive schematic and project authoring tool supporting adding/moving/deleting function blocks and periphery/virtual/memory flag references on schematic pages, connecting/disconnecting wires between pins, creating/deleting project tree items (Pages, Rooms, Categories, Memory Flags, Virtual I/Os), Undo/Redo (`Cmd+Z` / `Cmd+Shift+Z`), and lossless `.Loxone` XML serialization across ~81 Loxone Knowledge Base block types.

**Architecture:**
1. **`crates/loxone-core/src/block_schema.rs` & `crates/loxone-core/src/factory.rs`**: Expand the static schema catalog with `PAGE_SCHEMAS_4` (~40 additional Loxone Knowledge Base function block types across all categories), expose `page_block_schemas()`, and add deterministic UUID generation + factory constructors (`new_page_block`, `new_reference_block`, `new_page`, `new_room`, `new_category`, `new_tree_node`, `ensure_schema_pins_on_block`).
2. **`crates/loxone-core/src/writer.rs`**: Extend `ProjectEdit` with structural authoring mutations (`MovePageBlock`, `AddPageBlock`, `RemovePageBlock`, `AddWireLink`, `RemoveWireLink`, `AddPage`, `RemovePage`, `AddRoom`, `RemoveRoom`, `AddCategory`, `RemoveCategory`, `AddTreeNode`, `RemoveTreeNode`, `Batch`), implement reversible `apply_edit_with_inverse(project, edit) -> Result<ProjectEdit, LoxoneError>`, and extend `sync_raw_node` / `sync_co_children` to serialize added/removed/moved blocks, wires (`<In Input="..."/>` + `Nc`), pages, rooms, categories, and tree nodes back into `raw_root` while preserving untouched XML byte-for-byte.
3. **`crates/loxone-gui`**: Add a searchable Quick-Insert Picker (`insert_modal.rs`), interactive block dragging, pin-to-pin wire drawing, wire selection/deletion, and context menu on the schematic canvas (`canvas.rs`), project tree creation/deletion (`tree_panel.rs`), inspector delete/disconnect/place actions (`inspector.rs`), top-bar Undo/Redo/Add buttons (`top_bar.rs`), and an `undo_stack` / `redo_stack` transaction manager in `app.rs`.

**Tech Stack:** Rust 2021, `quick-xml` 0.37, `eframe` / `egui` 0.31.

---

## File Structure

- **Modify:** `crates/loxone-core/src/block_schema.rs` — Add `PAGE_SCHEMAS_4` (~40 standard Loxone Knowledge Base function block schemas) and `pub fn page_block_schemas() -> impl Iterator<Item = &'static BlockSchema>`.
- **Create:** `crates/loxone-core/src/factory.rs` — Loxone 8-4-4-16 UUID generator (`generate_loxone_uuid`), `new_page_block`, `new_reference_block`, `ensure_schema_pins_on_block`, `new_page`, `new_room`, `new_category`, `new_tree_node`.
- **Modify:** `crates/loxone-core/src/lib.rs` — Export `pub mod factory;` and re-export `page_block_schemas` and `apply_edit_with_inverse`.
- **Modify:** `crates/loxone-core/src/writer.rs` — Structural `ProjectEdit` variants, `apply_edit_with_inverse`, and lossless `RawNode` synchronization for added/removed/moved blocks, `<In>` wire links, pages, rooms, categories, and tree nodes.
- **Create:** `crates/loxone-gui/src/insert_modal.rs` — Searchable Quick-Insert Picker (`A` / `+ Add Block`) with Function Blocks tab (filterable by category & search query) and References tab (Periphery inputs/outputs, Virtual inputs/outputs, `Merker` memory flags).
- **Modify:** `crates/loxone-gui/src/canvas.rs` — `CanvasInteractionState`, `CanvasAction`, `screen_to_loxone`, `snap_to_loxone_grid`, interactive block dragging, pin-to-pin wire dragging with live Bézier preview, wire click selection & midpoint `✕` / `Delete` disconnection, and canvas right-click context menu.
- **Modify:** `crates/loxone-gui/src/tree_panel.rs` — Section header `+` buttons (`+ Page`, `+ Room`, `+ Category`, `+ Flag`, `+ VIn`, `+ VOut`) and `TreeAction` return enum.
- **Modify:** `crates/loxone-gui/src/inspector.rs` — `InspectorAction` enum (`Edit`, `NavigateToUuid`, `DeleteSelected`, `PlaceReferenceOnActivePage`, `DisconnectWire`), `📥 Place on Current Page` button for periphery/virtual/memory nodes, `🗑 Delete` button for page blocks and user-authored tree items, and `✕` wire disconnect buttons in `INPUTS & OUTPUTS`.
- **Modify:** `crates/loxone-gui/src/top_bar.rs` — `+ Add Block (A)`, `↶ Undo`, and `↷ Redo` buttons.
- **Modify:** `crates/loxone-gui/src/app.rs` & `crates/loxone-gui/src/main.rs` — `undo_stack` / `redo_stack`, `execute_edit`, `undo`, `redo`, keyboard shortcuts (`Cmd+Z`, `Cmd+Shift+Z`, `A`, `Delete`/`Backspace`, `Escape`), and wiring of `InsertModalState`, `CanvasAction`, `TreeAction`, and `InspectorAction`.

---

### Task 1: Complete Loxone KB Block Catalog & Element Factory (`crates/loxone-core/src/block_schema.rs`, `crates/loxone-core/src/factory.rs`, `crates/loxone-core/src/lib.rs`)

**Files:**
- Modify: `crates/loxone-core/src/block_schema.rs`
- Create: `crates/loxone-core/src/factory.rs`
- Modify: `crates/loxone-core/src/lib.rs`

- [ ] **Step 1: Write failing tests for the expanded block catalog (`page_block_schemas`) and element factory (`factory.rs`)**

Create `crates/loxone-core/src/factory.rs` with unit tests first, and wire `pub mod factory;` in `crates/loxone-core/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::{lookup_block_schema, page_block_schemas};
    use crate::model::PinKind;

    #[test]
    fn test_uuid_format_matches_loxone_8_4_4_16_lowercase_hex() {
        let u1 = generate_loxone_uuid();
        let u2 = generate_loxone_uuid();
        assert_ne!(u1, u2);
        let parts: Vec<&str> = u1.split('-').collect();
        assert_eq!(parts.len(), 4, "UUID must have 4 hyphen-separated segments: {u1}");
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 16);
        assert!(u1.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
        assert_eq!(u1, u1.to_ascii_lowercase());
    }

    #[test]
    fn test_expanded_page_block_catalog_has_over_75_types() {
        let count = page_block_schemas().count();
        assert!(
            count >= 75,
            "Expected at least 75 page block schemas in catalog, got {count}"
        );
        for required in [
            "Switch",
            "Pushbutton",
            "StairwellLight",
            "OffDelay",
            "DoubleClick",
            "ThresholdSwitch",
            "Hysteresis",
            "Limiter",
            "Average",
            "MinMax",
            "PID",
            "HeatingCurve",
            "PWM",
            "ClimateController",
            "MoodSwitch",
            "LightScene",
            "PresenceLight",
            "IntervalImpulse",
            "Stopwatch",
            "OperatingHoursCounter",
            "Schedule",
            "AlarmClock",
            "AccessCode",
            "FireEmergency",
            "WaterCentral",
            "PoolController",
            "SaunaController",
            "SolarController",
            "LoadManager",
            "Wallbox2",
            "SpotPriceOptimizer",
        ] {
            assert!(
                lookup_block_schema(required).is_some(),
                "Missing required KB block schema: {required}"
            );
        }
    }

    #[test]
    fn test_new_page_block_populates_default_attrs_and_pins() {
        let block = new_page_block("StairwellLight", "Hallway Stair Light", 128.0, 256.0);
        assert_eq!(block.block_type, "StairwellLight");
        assert_eq!(block.title, "Hallway Stair Light");
        assert_eq!(block.px, 128.0);
        assert_eq!(block.py, 256.0);
        assert!(block.pins.iter().any(|p| p.key == "Tr" && p.kind == PinKind::Input));
        assert!(block.pins.iter().any(|p| p.key == "Q" && p.kind == PinKind::Output));
        assert!(block.pins.iter().any(|p| p.key == "T" && p.default_value.is_some()));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-core factory`
Expected: Compilation error (`page_block_schemas`, `generate_loxone_uuid`, `new_page_block` not yet defined).

- [ ] **Step 3: Implement `PAGE_SCHEMAS_4` and `page_block_schemas()` in `crates/loxone-core/src/block_schema.rs` and `crates/loxone-core/src/factory.rs`**

In `crates/loxone-core/src/block_schema.rs`:
1. Add `static PAGE_SCHEMAS_4: &[BlockSchema]` containing 42 standard Loxone Knowledge Base function block schemas across all categories:
   - **Digital & Switching**: `Pushbutton`, `StairwellLight`, `OffDelay`, `DoubleClick`
   - **Analog & Math**: `ThresholdSwitch`, `Hysteresis`, `Limiter`, `Average`, `MinMax`, `ChangeRate`, `ValueSelector`, `Scale`, `Linearity`
   - **Shading & Windows**: `CentralJalousie`, `CentralGate`, `RoofWindowShading`, `WindowMonitor`
   - **Climate & Control**: `PID`, `HeatingCurve`, `PWM`, `TwoPointController`, `ClimateController`, `CeilingFan`, `Dehumidifier`
   - **Lighting & Audio**: `MoodSwitch`, `LightScene`, `PresenceLight`, `StereoExtension`
   - **Timing, Counters & Schedule**: `IntervalImpulse`, `Stopwatch`, `OperatingHoursCounter`, `Schedule`, `AlarmClock`
   - **Security, Safety & Access**: `AccessCode`, `FireEmergency`, `WaterCentral`
   - **Energy, Pool, Sauna & EV**: `PoolController`, `SaunaController`, `SolarController`, `LoadManager`, `Wallbox2`, `SpotPriceOptimizer`
2. Include `PAGE_SCHEMAS_4` in `lookup_block_schema(type_name: &str)`.
3. Add `pub fn page_block_schemas() -> impl Iterator<Item = &'static BlockSchema>` returning `BLOCK_SCHEMAS.iter().chain(PAGE_SCHEMAS_2.iter()).chain(PAGE_SCHEMAS_3.iter()).chain(PAGE_SCHEMAS_4.iter())`.

In `crates/loxone-core/src/factory.rs`, implement:
- `pub fn generate_loxone_uuid() -> String` (thread-safe atomic counter + timestamp + random seed producing lowercase `xxxxxxxx-xxxx-xxxx-xxxxxxxxxxxxxxxx` hex UUIDs).
- `pub fn new_page_block(type_name: &str, title: &str, px: f32, py: f32) -> PageBlock`: populates `uuid`, `block_type`, `title`, `short_desc`, `px`, `py`, `px2: px + 512.0`, `py2: py + 192.0`, `color: default_category_color(schema.category)`, default `raw_attributes` (`Type`, `Title`, `U`, `Px`, `Py`, `Cl`, plus schema `attributes` with non-empty defaults), and `pins` from `schema.pin_docs` and `schema.pin_params`.
- `pub fn ensure_schema_pins_on_block(block: &mut PageBlock)`: ensures every pin in `lookup_block_schema(&block.block_type)` (`pin_docs` + `pin_params`) exists in `block.pins` so any block (including blocks loaded from XML that omitted unconnected `<Co>` elements) exposes all connectible input and output pins.
- `pub fn new_reference_block(ref_type: &str, target_node: &TreeNode, px: f32, py: f32) -> PageBlock`: constructs an `InputRef`, `OutputRef`, `OutputRefLM`, or `Memory` `PageBlock` with `Ref` set to `target_node.uuid`, `Title` / `Desc` copied from `target_node`, and appropriate `I` / `O` connector pin(s) (reusing existing `<Co>` pin UUIDs from `target_node.pins` when present, or generating a fresh `<Co>` pin).
- `pub fn new_page(title: &str) -> Page`
- `pub fn new_room(title: &str) -> Place`
- `pub fn new_category(title: &str, cat_type: &str) -> Category`
- `pub fn new_tree_node(type_name: &str, title: &str) -> TreeNode`

- [ ] **Step 4: Run `cargo test -p loxone-core` to verify all schema and factory tests pass**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-core`
Expected: All existing tests plus the 3 new `factory` tests pass with 0 warnings.

- [ ] **Step 5: Commit Task 1**

```bash
git add crates/loxone-core/src/block_schema.rs crates/loxone-core/src/factory.rs crates/loxone-core/src/lib.rs
git commit -m "feat(core): expand Loxone KB block catalog to 83 types and add element factory"
```

---

### Task 2: Structural `ProjectEdit`, Reversible Undo/Redo & Lossless XML Sync (`crates/loxone-core/src/writer.rs`)

**Files:**
- Modify: `crates/loxone-core/src/writer.rs`
- Modify: `crates/loxone-core/src/lib.rs`

- [ ] **Step 1: Write failing tests for structural `ProjectEdit` variants, `apply_edit_with_inverse`, and XML round-tripping**

In `crates/loxone-core/src/writer.rs` (`mod tests`), add tests for:
1. `test_add_move_remove_page_block_and_undo_roundtrip`:
   - Parse `Klimczaki.Loxone`, create a `StairwellLight` block via `factory::new_page_block`, apply `ProjectEdit::AddPageBlock`, move it via `ProjectEdit::MovePageBlock`, serialize & reparse to verify the new block and coordinates are persisted in XML, then apply the returned inverse edits in reverse order and verify the page returns to its original block count.
2. `test_add_and_remove_wire_link_and_undo_roundtrip`:
   - Connect an output pin of one block to an input pin of another block via `ProjectEdit::AddWireLink`, serialize & reparse to verify `<In Input="..."/>` and `Nc` are written and indexed in `XrefIndex`, then apply the inverse (`ProjectEdit::RemoveWireLink`) and verify the wire link is removed.
3. `test_remove_block_cleans_up_dangling_wires_and_restores_on_undo`:
   - Connect Block A's output pin to Block B's input pin, then apply `ProjectEdit::RemovePageBlock` on Block A. Verify Block B's incoming wire to Block A is automatically removed, and applying the returned inverse `ProjectEdit::Batch` restores both Block A and Block B's wire link.
4. `test_add_and_remove_page_room_category_and_tree_node_roundtrip`:
   - Add a new `Page`, `Place`, `Category`, and `Merker` `TreeNode`, serialize & reparse to verify all 4 appear in the reparsed `LoxoneProject`, then apply their inverse edits and verify removal.

- [ ] **Step 2: Run tests to verify they fail**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-core writer`
Expected: Fails to compile because the new `ProjectEdit` variants and `apply_edit_with_inverse` do not exist yet.

- [ ] **Step 3: Implement structural `ProjectEdit` variants, `apply_edit_with_inverse`, and `RawNode` XML sync in `crates/loxone-core/src/writer.rs`**

1. Extend `ProjectEdit`:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub enum ProjectEdit {
       SetControlAttr { uuid: String, attr: String, value: Option<String> },
       SetPinDefault { block_uuid: String, pin_key: String, value: Option<String> },
       SetChildTagAttr { block_uuid: String, tag_name: String, attr: String, value: Option<String> },
       MovePageBlock { page_uuid: String, block_uuid: String, px: f32, py: f32 },
       AddPageBlock { page_uuid: String, block: PageBlock },
       RemovePageBlock { page_uuid: String, block_uuid: String },
       AddWireLink { target_block_uuid: String, target_pin_key: String, source_pin_uuid: String },
       RemoveWireLink { target_block_uuid: String, target_pin_key: String, source_pin_uuid: String },
       AddPage { page: Page },
       RemovePage { page_uuid: String },
       AddRoom { room: Place },
       RemoveRoom { room_uuid: String },
       AddCategory { category: Category },
       RemoveCategory { category_uuid: String },
       AddTreeNode { section: TreeSectionKind, parent_uuid: Option<String>, node: TreeNode },
       RemoveTreeNode { node_uuid: String },
       Batch(Vec<ProjectEdit>),
   }
   ```
2. Implement `pub fn apply_edit_with_inverse(project: &mut LoxoneProject, edit: &ProjectEdit) -> Result<ProjectEdit, LoxoneError>`:
   - Every mutation records the exact prior state and returns the inverse `ProjectEdit` (and `apply_edit` delegates to `apply_edit_with_inverse(project, edit).map(|_| ())`).
   - For `AddWireLink`: if `target_pin_key` is not yet in `block.pins`, call `crate::factory::ensure_schema_pins_on_block(block)` or insert a new `ConnectorPin` with `PinKind::Input` and a fresh UUID; append `WireLink { target_pin_uuid: source_pin_uuid, flags: None }` if not already linked.
   - For `RemovePageBlock`: collect any incoming `WireLink`s on other blocks across all pages whose `target_pin_uuid` belongs to one of the removed block's `pins`, remove those links and the block itself, and return a `ProjectEdit::Batch` that first re-adds the block (`AddPageBlock`) and then re-adds each removed wire (`AddWireLink`).
   - Always rebuild `project.xref = XrefIndex::build(project);` after applying edits.
3. Extend `sync_raw_node` and `sync_co_children` in `crates/loxone-core/src/writer.rs`:
   - In `sync_co_children`:
     - Update existing `<Co>` nodes and append new `<Co>` `RawNode`s for any `ConnectorPin` in `pins` whose `uuid` or `key` wasn't in `existing_children`.
     - Check whether `co`'s existing `<In>` children match `pin.incoming_links`. Only when they differ (or on a newly created `<Co>`), update `co.children` with `<In Input="..."/>` nodes and update `Nc` (`Some(&pin.incoming_links.len().to_string())` when non-empty, `None` when empty). This guarantees 100% byte-for-byte preservation on untouched `<Co>` tags in `Klimczaki.Loxone`.
   - Helper functions to convert typed model structs into `RawNode`s when newly added:
     - `page_block_to_raw_node(block: &PageBlock) -> RawNode`
     - `page_to_raw_node(page: &Page) -> RawNode`
     - `place_to_raw_node(place: &Place) -> RawNode`
     - `category_to_raw_node(cat: &Category) -> RawNode`
     - `tree_node_to_raw_node(node: &TreeNode) -> RawNode`
   - In `sync_raw_node`:
     - When `node` is `<C Type="Page">` and matches `page` in `pages_by_uuid`: sync `Title`, sync `<IoData>`, retain `<C>` children whose `U` is in `page.blocks`, sync retained `<C>` children, and append `page_block_to_raw_node(block)` for any `PageBlock` in `page.blocks` not yet in `node.children`.
     - When `node` is `<C Type="Program">`: retain `<C Type="Page">` children whose `U` is in `pages_by_uuid`, sync existing children, and append `page_to_raw_node(page)` for any newly added `Page` in `project.pages`.
     - When `node` is `<C Type="CategoryCaption">`: retain `<C Type="Category">` children whose `U` is in `project.categories`, sync existing, and append `category_to_raw_node(cat)` for newly added categories.
     - When `node` is `<C Type="PlaceCaption">`: retain `<C Type="Place">` children whose `U` is in `project.places`, sync existing, and append `place_to_raw_node(place)` for newly added rooms.
     - When `node` matches a `tree_node` in `tree_nodes_by_uuid`: retain `<C>` children whose `U` is in `tree_node.children`, sync existing, and append `tree_node_to_raw_node(child)` for newly added child `TreeNode`s.

- [ ] **Step 4: Run `cargo test -p loxone-core` to verify all writer and lossless round-trip tests pass**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-core`
Expected: All `loxone-core` tests (including `test_lossless_roundtrip_unmodified_klimczaki` and the 4 new structural authoring tests) pass with 0 warnings.

- [ ] **Step 5: Commit Task 2**

```bash
git add crates/loxone-core/src/writer.rs crates/loxone-core/src/lib.rs
git commit -m "feat(core): add structural ProjectEdit mutations, undo/redo inverses, and XML sync"
```

---

### Task 3: Quick-Insert Modal (`crates/loxone-gui/src/insert_modal.rs`)

**Files:**
- Create: `crates/loxone-gui/src/insert_modal.rs`
- Modify: `crates/loxone-gui/src/main.rs`

- [ ] **Step 1: Write failing tests for `InsertModalState` filtering and element creation**

Create `crates/loxone-gui/src/insert_modal.rs` with unit tests verifying:
1. Searching `"stair"` filters `matching_block_schemas` to include `StairwellLight`.
2. Category filter (`Some("Climate & Control")`) returns only schemas in `"Climate & Control"`.
3. Searching reference candidates against `Klimczaki.Loxone` finds periphery I/Os, virtual I/Os, and `Merker` memory flags.

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-gui insert_modal`
Expected: Fails until `InsertModalState` and `show_insert_modal` are implemented.

- [ ] **Step 3: Implement `InsertModalState`, `InsertModalSelection`, and `show_insert_modal` in `crates/loxone-gui/src/insert_modal.rs`**

Implement:
- `pub enum InsertTab { FunctionBlocks, References }`
- `pub enum InsertModalSelection { NewFunctionBlock { type_name: String, title: String, px: f32, py: f32 }, NewReferenceBlock { ref_type: String, target_uuid: String, px: f32, py: f32 } }`
- `pub struct InsertModalState`:
  - `pub is_open: bool`
  - `pub query: String`
  - `pub tab: InsertTab`
  - `pub selected_category: Option<String>`
  - `pub selected_index: usize`
  - `pub target_pos: Option<(f32, f32)>` (Loxone `(px, py)` coordinates where the block will be inserted)
  - `pub focus_requested: bool`
- `pub fn open_at(&mut self, pos: Option<(f32, f32)>)`
- `pub fn show_insert_modal(ctx: &egui::Context, state: &mut InsertModalState, project: Option<&LoxoneProject>) -> Option<InsertModalSelection>`:
  - Uses `egui::Window` with `theme::BG_PANEL` background and crisp border (matching the Command Palette fix so no dark backdrop dims the screen).
  - Top tab toggle: `🧩 Function Blocks (83)` vs `📥 Periphery / Flags / Virtual I/O`.
  - In `FunctionBlocks` tab: horizontal category filter chips (`All`, plus each unique category from `page_block_schemas()`) + search input + scrollable list showing block display name, `[TypeName]`, category badge, and summary. Pressing `Enter` or clicking a row returns `Some(InsertModalSelection::NewFunctionBlock { ... })` and closes the modal.
  - In `References` tab: lists placeable nodes from `Periphery`, `VirtualIO`, and `MemoryFlags` (`DigitalIn`/`VirtualIn` -> `InputRef`, `DigitalOut`/`AnalogOut`/`Relay`/`VirtualOut`/`VirtualOutCmd` -> `OutputRef`, `Merker` -> `Memory`) with search filtering. Clicking a row or pressing `Enter` returns `Some(InsertModalSelection::NewReferenceBlock { ... })`.

- [ ] **Step 4: Run `cargo test -p loxone-gui insert_modal`**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-gui insert_modal`
Expected: All `insert_modal` tests pass.

- [ ] **Step 5: Commit Task 3**

```bash
git add crates/loxone-gui/src/insert_modal.rs crates/loxone-gui/src/main.rs
git commit -m "feat(gui): add searchable Quick-Insert modal for function blocks and references"
```

---

### Task 4: Interactive Canvas Authoring (`crates/loxone-gui/src/canvas.rs`)

**Files:**
- Modify: `crates/loxone-gui/src/canvas.rs`

- [ ] **Step 1: Write failing tests for coordinate inverse mapping, grid snapping, and wire hit-testing**

In `crates/loxone-gui/src/canvas.rs` (`mod tests`), add unit tests:
1. `test_screen_to_loxone_is_exact_inverse_of_loxone_to_screen`:
   - Verify `viewport.screen_to_loxone(origin, viewport.loxone_to_screen(origin, 512.0, 256.0))` equals `(512.0, 256.0)` across zoom levels (`1.0`, `1.5`, `0.65`) and pan offsets.
2. `test_snap_to_loxone_grid_snaps_to_64_unit_multiples`:
   - Verify `snap_to_loxone_grid(0.0) == 0.0`, `snap_to_loxone_grid(31.0) == 0.0`, `snap_to_loxone_grid(33.0) == 64.0`, `snap_to_loxone_grid(500.0) == 512.0`.
3. `test_point_to_cubic_bezier_distance_detects_wire_click`:
   - Verify a point near the midpoint of `cubic_bezier_points` is within the `8.0` px hit threshold, while a far point is outside.

- [ ] **Step 2: Run tests to verify they fail**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-gui canvas`
Expected: Fails to compile because `screen_to_loxone`, `snap_to_loxone_grid`, and `point_to_bezier_distance` do not exist yet.

- [ ] **Step 3: Implement interactive block dragging, pin-to-pin wiring, wire selection/deletion, and right-click context menu in `crates/loxone-gui/src/canvas.rs`**

1. Add `CanvasAction` and `CanvasInteractionState`:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub enum CanvasAction {
       SelectBlock(String),
       ClearSelection,
       MoveBlock { page_uuid: String, block_uuid: String, px: f32, py: f32 },
       ConnectPins { target_block_uuid: String, target_pin_key: String, source_pin_uuid: String },
       DisconnectWire { target_block_uuid: String, target_pin_key: String, source_pin_uuid: String },
       DeleteBlock { page_uuid: String, block_uuid: String },
       OpenInsertModalAt { px: f32, py: f32 },
   }
   ```
2. Add `screen_to_loxone(&self, canvas_origin: Pos2, pos: Pos2) -> (f32, f32)` and `pub fn snap_to_loxone_grid(val: f32) -> f32` (`(val / 64.0).round() * 64.0`).
3. Update `compute_block_layout`:
   - Include unconnected schema pins from `lookup_block_schema(&block.block_type)` (`pin_docs`) when `!compact_pins` (or when `viewport.interaction.wiring_from.is_some()`), so users can wire to any input or output pin defined in the block's Loxone schema even if it wasn't previously connected in XML!
4. Implement interactive behaviors in `show_canvas`:
   - **Block Dragging**: Dragging a block header/card updates `viewport.interaction.dragging_block` (`BlockDragState { block_uuid, start_px, start_py, current_px, current_py }`), renders the block and its connected wires at `(current_px, current_py)` in real time, and emits `CanvasAction::MoveBlock` snapped to `64.0` Loxone units when the drag is released.
   - **Pin-to-Pin Wiring**: Hovering within `8.0` px of any Input or Output pin dot highlights it; dragging (or clicking) from an Output pin to an Input pin (or Input pin to Output pin) draws an animated cubic Bézier preview curve from the source pin to the pointer and emits `CanvasAction::ConnectPins` on drop/click onto a compatible pin of opposite polarity.
   - **Wire Selection & Disconnection**: Clicking within `7.0` px of a rendered cubic Bézier wire selects it (`viewport.interaction.selected_wire`), highlights the wire in bright accent color with a clickable `✕` delete badge at its midpoint, and clicking the `✕` badge (or pressing `Delete`/`Backspace`) emits `CanvasAction::DisconnectWire`.
   - **Canvas Context Menu**: Right-clicking empty canvas opens a popup menu (`➕ Add Function Block / Reference here...`) that emits `CanvasAction::OpenInsertModalAt { px, py }`; right-clicking a block offers `🗑 Delete Block`.

- [ ] **Step 4: Run `cargo test -p loxone-gui canvas`**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-gui canvas`
Expected: All `canvas` tests pass.

- [ ] **Step 5: Commit Task 4**

```bash
git add crates/loxone-gui/src/canvas.rs
git commit -m "feat(gui): add interactive block dragging, pin-to-pin wiring, and wire selection on canvas"
```

---

### Task 5: Project Tree Authoring, Inspector Actions & Undo/Redo App Wiring (`crates/loxone-gui/src/tree_panel.rs`, `crates/loxone-gui/src/inspector.rs`, `crates/loxone-gui/src/top_bar.rs`, `crates/loxone-gui/src/app.rs`, `crates/loxone-gui/src/main.rs`)

**Files:**
- Modify: `crates/loxone-gui/src/tree_panel.rs`
- Modify: `crates/loxone-gui/src/inspector.rs`
- Modify: `crates/loxone-gui/src/top_bar.rs`
- Modify: `crates/loxone-gui/src/app.rs`
- Modify: `crates/loxone-gui/src/main.rs`

- [ ] **Step 1: Write failing tests in `app.rs` for Undo/Redo stack, inserting blocks/references, connecting wires, and creating tree items**

In `crates/loxone-gui/src/app.rs` (`#[cfg(test)] mod tests`), add tests:
1. `test_app_execute_edit_and_undo_redo_updates_dirty_and_restores_state`:
   - Load `Klimczaki.Loxone` into `LoxoneApp`, execute an `AddPageBlock` edit via `app.execute_edit(...)`, verify `app.is_dirty == true` and `app.can_undo() == true`, call `app.undo()`, verify the block is removed and `app.can_redo() == true`, call `app.redo()`, verify the block is back.

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test -p loxone-gui app`
Expected: Fails because `execute_edit`, `undo`, `redo`, `can_undo`, `can_redo` are not yet defined on `LoxoneApp`.

- [ ] **Step 3: Update `tree_panel.rs`, `inspector.rs`, `top_bar.rs`, `app.rs`, and `main.rs`**

1. **`crates/loxone-gui/src/tree_panel.rs`**:
   - Add `pub enum TreeAction { SelectPage(usize), SelectNode(String), CreatePage, CreateRoom, CreateCategory, CreateMemoryFlag, CreateVirtualIn, CreateVirtualOut, PlaceOnPage(String) }`.
   - In `show_tree_panel`, add compact `+` action buttons in the section headers (`+ Page` next to `SCHEMATIC PAGES`, `+ Flag` / `+ VIn` / `+ VOut` in `PERIPHERY & SYSTEM`, `+ Room` next to `ROOMS`, `+ Cat` next to `CATEGORIES`) and a right-click context menu on placeable periphery/virtual/memory nodes (`📥 Place on Current Page`).
2. **`crates/loxone-gui/src/inspector.rs`**:
   - Replace `InspectorOutput` fields with clear actions:
     - `pub pending_edits: Vec<ProjectEdit>`
     - `pub navigate_to_uuid: Option<String>`
     - `pub delete_uuid: Option<String>`
     - `pub place_on_page_uuid: Option<String>`
   - In the Identity Card:
     - For `ProjectObject::PageBlock`: render a `🗑 Delete Block` button (`delete_uuid = Some(block.uuid.clone())`).
     - For `ProjectObject::TreeNode`: if the node is placeable on a schematic page (`Merker`, `VirtualIn`, `VirtualOut`, `VirtualOutCmd`, `DigitalIn`, `DigitalOut`, `AnalogIn`, `AnalogOut`, `Relay`, `Sensor`), render a `📥 Place on Current Page` button (`place_on_page_uuid = Some(node.uuid.clone())`). If the node is user-authored (`Merker`, `VirtualIn`, `VirtualOut`, `VirtualOutCmd`), also render a `🗑 Delete` button.
     - For `ProjectObject::Page`, `ProjectObject::Place`, `ProjectObject::Category`: render editable Title/Name field + `🗑 Delete` button.
   - In `INPUTS & OUTPUTS` (`show_pins_section`):
     - Next to each incoming wire link row (`⬅ source — pin`), add a compact `✕` disconnect button that pushes `ProjectEdit::RemoveWireLink { target_block_uuid, target_pin_key, source_pin_uuid }` to `pending_edits`.
3. **`crates/loxone-gui/src/top_bar.rs`**:
   - Add `AddBlock`, `Undo`, and `Redo` variants to `TopBarAction`.
   - Render `➕ Add Block (A)`, `↶ Undo`, and `↷ Redo` buttons in the top bar (with `can_undo` / `can_redo` enabled state).
4. **`crates/loxone-gui/src/app.rs` & `crates/loxone-gui/src/main.rs`**:
   - Register `mod insert_modal;` in `main.rs`.
   - Add `pub undo_stack: Vec<ProjectEdit>`, `pub redo_stack: Vec<ProjectEdit>`, and `pub insert_modal: InsertModalState` to `LoxoneApp`.
   - Implement `pub fn execute_edit(&mut self, edit: ProjectEdit)` using `apply_edit_with_inverse`, pushing the inverse edit onto `self.undo_stack`, clearing `self.redo_stack`, and marking `self.is_dirty = true`.
   - Implement `pub fn undo(&mut self)` and `pub fn redo(&mut self)`.
   - Wire keyboard shortcuts (`Cmd+Z` / `Ctrl+Z` -> `undo()`, `Cmd+Shift+Z` / `Ctrl+Shift+Z` -> `redo()`, `A` -> open Quick-Insert modal, `Delete` / `Backspace` -> delete selected wire or selected page block) and handle all `CanvasAction`, `TreeAction`, `InspectorOutput`, and `InsertModalSelection` events.

- [ ] **Step 4: Run full workspace test suite and build check**

Run: `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH" && cargo test --workspace && cargo build --workspace`
Expected: All unit & integration tests across `loxone-core` and `loxone-gui` pass with 0 compiler warnings.

- [ ] **Step 5: Commit Task 5**

```bash
git add crates/loxone-gui/src/tree_panel.rs crates/loxone-gui/src/inspector.rs crates/loxone-gui/src/top_bar.rs crates/loxone-gui/src/app.rs crates/loxone-gui/src/main.rs
git commit -m "feat(gui): wire project tree creation, inspector actions, insert modal, and undo/redo"
```
