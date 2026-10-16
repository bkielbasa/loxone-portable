# Loxone Portable — Phase 3: Interactive Schematic & Project Authoring Design Spec

- **Date:** 2026-10-16
- **Status:** Approved
- **Scope:** Full Interactive Schematic & Project Authoring (`loxone-core` & `loxone-gui`)

---

## 1. Overview & Goals

Transform `loxone-portable` from a schematic viewer + property editor into a full **Interactive Schematic & Project Authoring Tool** capable of creating, moving, wiring, disconnecting, and deleting function blocks, periphery/virtual I/O reference bars, and memory flag bars on schematic pages, as well as creating and deleting pages, rooms, categories, virtual I/Os, and memory flags in the project tree — with reversible Undo/Redo (`Cmd+Z` / `Cmd+Shift+Z`) and lossless `.Loxone` XML serialization.

### Non-Goals
- Live network communication or Miniserver online deployment (remains an offline `.Loxone` project authoring tool).
- Proprietary encrypted Loxone binary formats (`.LoxCC`); works with standard `.Loxone` XML project files.

---

## 2. Architecture: Unified Reversible `ProjectEdit` Command Log + Schema-Driven Factory

### 2.1 Complete Loxone Knowledge Base Block Catalog (`crates/loxone-core/src/block_schema.rs`)
- Expand `ALL_BLOCK_SCHEMAS` from the 41 block types present in `Klimczaki.Loxone` to **76 Loxone Config function block types** by adding 35 standard Loxone Knowledge Base blocks across all functional categories:
  1. **Switch & Pushbutton:** `Pushbutton` (Pushbutton), `MultiClick` (Multi-Click), `StairwellLight` (Stairwell Light Switch), `OffDelay` (Off-Delay), `DoubleClick` (Double-Click)
  2. **Analog & Threshold:** `ThresholdSwitch` (Threshold Switch), `Hysteresis` (Hysteresis), `Limiter` (Limiter), `Average` (Average), `MinMax` (Min/Max), `ChangeRate` (Rate of Change), `ValueSelector` (Value Selector), `Scale` (Scale / Multiplier), `Linearity` (Linearity Correction)
  3. **Shading & Gates:** `CentralJalousie` (Central Shading), `CentralGate` (Central Gate), `RoofWindowShading` (Roof Window Shading), `WindowMonitor` (Window & Door Monitor)
  4. **Climate, Heating & Cooling:** `PID` (PID Controller), `HeatingCurve` (Heating Curve), `PWM` (PWM), `TwoPointController` (2-Point Controller), `ClimateController` (Climate Controller), `CeilingFan` (Ceiling Fan Controller), `Dehumidifier` (Dehumidifier)
  5. **Lighting & Audio:** `MoodSwitch` (Lighting Mood Switch), `LightScene` (Light Scene), `PresenceLight` (Presence Lighting), `StereoExtension` (Stereo Controller)
  6. **Timing & Counters:** `IntervalImpulse` (Interval Impulse), `Stopwatch` (Stopwatch), `OperatingHoursCounter` (Operating Hours Counter), `Schedule` (Schedule), `AlarmClock` (Alarm Clock)
  7. **Access, Safety & Energy:** `AccessCode` (NFC / Access Code), `Intercom` (Intercom), `FireEmergency` (Fire & Water Alarm), `WaterCentral` (Central Water), `PoolController` (Pool Controller), `SaunaController` (Sauna Controller), `SolarController` (Solar Controller), `LoadManager` (Load Manager), `Wallbox2` (Wallbox), `SpotPriceOptimizer` (Spot Price Optimizer)
- Provide a public `all_block_schemas() -> &'static [BlockSchema]` accessor so the GUI can browse, filter, and instantiate any block type in the catalog.

### 2.2 Element & Reference Bar Factory (`crates/loxone-core/src/factory.rs`)
- **UUID Generation (`generate_loxone_uuid() -> String`):**
  - Produces Loxone-compatible UUID strings formatted as `xxxxxxxx-xxxx-xxxx-ffffffffxxxx` (lowercase hex segments matching Loxone Config's 8-4-4-12 format).
- **Schema-Driven Function Block Instantiation (`instantiate_block_from_schema`):**
  - `instantiate_block_from_schema(type_name: &str, title: Option<&str>, px: f32, py: f32) -> Result<PageBlock, LoxoneError>`
  - Looks up `get_block_schema(type_name)`.
  - Computes default dimensions: width `512.0` (`px2 = px + 512.0`), height `192.0_f32.max(96.0 + max_io_rows as f32 * 48.0)` rounded up to a multiple of `64.0`.
  - Derives category color `Cl` RGB string from `schema.category`.
  - Populates `raw_attributes` (`Type`, `V="178"`, `U`, `Title`, `Px`, `Py`, `Px2`, `Py2`, `Cl`, plus any `AttributeSchema` with a non-empty `default_value`).
  - Materializes `ConnectorPin` entries (`<Co>`) for all `schema.inputs` (`PinKind::Input`), `schema.outputs` (`PinKind::Output`), and `schema.pin_params` (`PinKind::Parameter` with `default_value = Some(param.default_value)`).
- **Periphery / Virtual I/O Reference Bar Instantiation (`instantiate_periphery_ref`):**
  - `PeripheryRefKind`: `InputRef`, `OutputRef`, `OutputRefLM`.
  - `instantiate_periphery_ref(target: &TreeNode, kind: PeripheryRefKind, px: f32, py: f32) -> PageBlock`
  - Creates a reference bar of size `2112.0 x 192.0` (`Px2 = Px + 2112`, `Py2 = Py + 192`), `Cl="0,0,0"`, `Ref="<target.uuid>"`, `Title="<target.title>"`, inheriting `Room` and `Cat` from `target`.
  - For `InputRef`: creates output pins `<Co K="AQ">` and `<Co K="Q">`.
  - For `OutputRef` / `OutputRefLM`: creates input pins `<Co K="AI">` and `<Co K="AQ">` (plus output pins `<Co K="AQ">` / `<Co K="Q">` when `OutputRefLM`).
- **Memory Flag Bar Instantiation (`instantiate_memory_bar`):**
  - `instantiate_memory_bar(merker: Option<&TreeNode>, title: &str, px: f32, py: f32) -> PageBlock`
  - Creates `<C Type="Memory" V="178" U="..." Title="..." Px="..." Py="..." Px2="..." Py2="..." Cl="238,238,238" Nio="3" WF="16" Tp="0">` (plus `Ref="<merker.uuid>"` if linked to a `Merker` tree node) with `<Co K="Input">` (`PinKind::Input`), `<Co K="AQ">` (`PinKind::Output`), and `<Co K="Q">` (`PinKind::Output`).
- **Project Tree Object Factories:**
  - `instantiate_page(title: &str) -> Page`
  - `instantiate_room(title: &str) -> Place`
  - `instantiate_category(title: &str, cat_type: &str, color: &str) -> Category`
  - `instantiate_tree_node(object_type: &str, title: &str, section: TreeSectionKind) -> TreeNode` (supports `Merker`, `VirtualIn`, `VirtualOut`, `VirtualOutCmd`).

### 2.3 Structural `ProjectEdit`, Reversible Undo/Redo & Lossless XML Sync (`crates/loxone-core/src/writer.rs`)
- **Extended `ProjectEdit` Enum:**
  - Existing property edits (`SetMetadataAttr`, `SetPageTitle`, `SetRoomAttr`, `SetCategoryAttr`, `SetBlockAttr`, `RemoveBlockAttr`, `SetBlockPinDefault`, `SetTreeNodeAttr`, `RemoveTreeNodeAttr`).
  - New structural edits:
    - `MovePageBlock { page_uuid: String, block_uuid: String, px: f32, py: f32, px2: f32, py2: f32 }`
    - `AddPageBlock { page_uuid: String, block: PageBlock }`
    - `RemovePageBlock { page_uuid: String, block_uuid: String }`
    - `AddWireLink { page_uuid: String, target_block_uuid: String, target_pin_key: String, link: WireLink }` (automatically finds or materializes the `<Co K="target_pin_key">` pin on `target_block_uuid` and appends `link` if not already present)
    - `RemoveWireLink { page_uuid: String, target_block_uuid: String, target_pin_uuid: String, source_pin_uuid: String }`
    - `AddPage { page: Page }`
    - `RemovePage { page_uuid: String }`
    - `AddRoom { place: Place }`
    - `RemoveRoom { uuid: String }`
    - `AddCategory { category: Category }`
    - `RemoveCategory { uuid: String }`
    - `AddTreeNode { section: TreeSectionKind, parent_uuid: Option<String>, node: TreeNode }`
    - `RemoveTreeNode { uuid: String }`
    - `Batch(Vec<ProjectEdit>)` (allows compound operations—such as removing a block along with any wires connected to its output pins across the page—to execute and undo atomically as a single undo step).
- **Reversible Execution (`apply_edit_with_inverse`):**
  - `apply_edit_with_inverse(project: &mut LoxoneProject, edit: ProjectEdit) -> Result<ProjectEdit, LoxoneError>`
  - Mutates the in-memory `LoxoneProject` and returns the exact inverse `ProjectEdit` that restores the previous state when applied.
  - Note: When `RemovePageBlock` executes, it captures all `RemoveWireLink` edits for other blocks on the page whose pins had incoming links from the removed block's output pins, plus the removed `PageBlock` itself, returning a `ProjectEdit::Batch` that restores both the block and those outgoing wires on Undo!
- **Lossless XML Tree Sync (`sync_raw_node` & `sync_co_children`):**
  - **Connector Pins & Wires (`sync_co_children`):**
    - Updates existing `<Co>` children and appends newly materialized `<Co>` pins.
    - Synchronizes `pin.incoming_links` into `<In Input="<source_pin_uuid>"/>` child `RawNode`s inside each `<Co>`.
    - Sets `Nc="<count>"` on `<Co>` when `!pin.incoming_links.is_empty()` and removes `Nc` when `pin.incoming_links.is_empty()`.
  - **Page Blocks (`<C Type="Page">`):**
    - Updates existing `<C>` block children in place, removes `<C>` block children that were deleted from `page.blocks`, and appends new `RawNode` `<C>` elements for newly added `PageBlock`s.
  - **Pages (`<C Type="Program">`):**
    - Updates existing `<C Type="Page">` nodes, removes deleted pages, and appends newly created `<C Type="Page">` nodes inside `<C Type="Program">`.
  - **Rooms (`<C Type="PlaceCaption">`), Categories (`<C Type="CategoryCaption">`), and Tree Nodes (`MemoryCaption`, `VirtualInCaption`, `VirtualOutCaption`, etc.):**
    - Synchronizes additions and removals directly into their corresponding container nodes in `raw_document`.

---

## 3. Interactive Canvas Authoring (`crates/loxone-gui/src/canvas.rs` & `insert_modal.rs`)

### 3.1 Canvas State & Returned Actions
- Extend `CanvasState` with:
  - `dragging_block: Option<BlockDragState>` (`block_uuid`, `start_px`, `start_py`, `orig_px`, `orig_py`, `width`, `height`)
  - `wiring_from: Option<WiringSource>` (`block_uuid`, `pin_uuid`, `pin_key`, `pin_kind: PinKind`)
  - `selected_wire: Option<SelectedWire>` (`page_uuid`, `target_block_uuid`, `target_pin_uuid`, `source_pin_uuid`)
- Extend `CanvasAction` with:
  - `OpenInsertModal { world_pos: (f32, f32) }`
  - `CommitEdit(ProjectEdit)`
  - `PreviewMoveBlock { page_uuid: String, block_uuid: String, px: f32, py: f32, px2: f32, py2: f32 }` (updates live position during drag; `CommitEdit` is emitted on drag release with the original pre-drag position restored before applying the final `MovePageBlock` so the undo stack records a single clean step).

### 3.2 Drag-to-Move Blocks with Grid Snapping
- Dragging a block card moves it smoothly in world coordinates.
- On mouse release, `px` and `py` snap to Loxone's `64.0`-unit schematic grid (`(val / 64.0).round() * 64.0`), preserving block width (`px2 - px`) and height (`py2 - py`), and emit a single reversible `ProjectEdit::MovePageBlock`.

### 3.3 Interactive Pin-to-Pin Wiring & Auto-Uncompact Pins
- **Pin Hit-Testing & Hover Feedback:**
  - Every visible input and output pin dot (`radius = 6.0 * zoom` hit target) responds to hover and click/drag.
  - Hovering a pin dot draws a glowing ring and tooltip with the pin key and KB description.
- **Starting & Completing a Wire:**
  - Clicking or dragging from an Output pin dot (or Input pin dot) sets `state.wiring_from = Some(WiringSource { ... })`.
  - While `state.wiring_from.is_some()`:
    - **Auto-Uncompact All Page Blocks:** All blocks on the current page temporarily render **all** input and output pins defined in their `BlockSchema` (ignoring `compact_pins`), plus any existing `<Co>` pins, so the user can wire into any unused input pin.
    - A live cubic Bézier wire is drawn from the source pin dot to the mouse cursor (snapping to compatible target pin dots on hover).
    - Clicking or releasing on a compatible pin dot of the opposite direction (Output → Input or Input → Output) emits `ProjectEdit::AddWireLink { page_uuid, target_block_uuid, target_pin_key, link: WireLink { source_pin_uuid, flags: None } }` and clears `wiring_from`.
    - Pressing `Escape` or clicking empty canvas cancels `wiring_from`.

### 3.4 Wire Selection & Disconnection + Block Deletion
- **Wire Hit-Testing:**
  - Clicking within `6.0` screen pixels of any rendered cubic Bézier wire selects that wire (`state.selected_wire = Some(...)`) and clears `selected_uuid`.
  - Selected wires render with a highlighted cyan/amber stroke (`2.8` px) and show a small `✕ Disconnect` badge at their midpoint.
- **Disconnecting Wires:**
  - Pressing `Delete` or `Backspace` while a wire is selected, clicking the midpoint `✕ Disconnect` badge, right-clicking a wire, or clicking `✕` next to any wire in the Right Inspector's **Inputs & Outputs** section emits `ProjectEdit::RemoveWireLink`.
- **Deleting Blocks:**
  - Pressing `Delete` or `Backspace` while a canvas block is selected (when no text input has keyboard focus), or clicking `🗑 Delete Block` in the Right Inspector, removes the block and all wires connected to it as an atomic undoable `ProjectEdit`.

### 3.5 Quick-Insert Modal (`crates/loxone-gui/src/insert_modal.rs`)
- Triggered by:
  - Pressing `A` when the canvas is active (places at the center of the current viewport).
  - Right-clicking empty canvas space (places at the clicked world coordinates).
  - Clicking `+ Add Block (A)` in the top toolbar.
- Modal UI (using `Order::Middle` backdrop + `Order::Foreground` window):
  - Search input with instant fuzzy filtering across all items.
  - Category filter pills (`All`, `Periphery & Flags`, `Lighting`, `Shading`, `Climate`, `Logic`, `Analog`, `Timing`, `Audio`, `Access & Safety`, `Energy`, `System`).
  - **Function Blocks Section:** All 76 `BlockSchema` entries with category badge, type name, display title, and description. Selecting one instantiates the block via `instantiate_block_from_schema` and commits `ProjectEdit::AddPageBlock`.
  - **Periphery, Virtual I/O & Memory Flags Section:** Lists project `Periphery` and `System` tree items (`Merker`, `VirtualIn`, `VirtualOut`, digital/analog hardware inputs & outputs, ` Note`). Selecting one instantiates an `InputRef`, `OutputRef`, or `Memory` bar via `instantiate_periphery_ref` / `instantiate_memory_bar` and commits `ProjectEdit::AddPageBlock`.

---

## 4. Project Tree & Inspector Authoring (`crates/loxone-gui/src/tree_panel.rs`, `inspector.rs`, `app.rs`)

### 4.1 Left Tree Creation Controls (`tree_panel.rs`)
- Each section header in the Left Project Tree includes a compact `+` button:
  - **Pages (`+ Page`):** Creates a new `Page` (`"New Page N"`), switches the canvas to it, and selects it in the Right Inspector so its title can be edited immediately.
  - **Rooms (`+ Room`):** Creates a new `Place` (`"New Room N"`) and selects it in the Right Inspector.
  - **Categories (`+ Category`):** Creates a new `Category` (`"New Category N"`) and selects it in the Right Inspector.
  - **Periphery (`+ Virtual In` / `+ Virtual Out`):** Appends a new `VirtualIn` or `VirtualOut` node under `VirtualInCaption` / `VirtualOutCaption` (or root of `periphery_tree` if caption not present) and selects it.
  - **System (`+ Memory Flag`):** Appends a new `Merker` node under `MemoryCaption` in `system_tree` and selects it.

### 4.2 Inspector Authoring & `📥 Place on Current Page` (`inspector.rs`)
- **Place on Current Page:**
  - When a `TreeNode` in `Periphery` or `System` is selected in the Right Inspector and a schematic page is open, render action buttons in the Identity Card:
    - For `Merker` / `Memory`: `📥 Place Memory Bar on Page`
    - For Inputs (`VirtualIn`, `DigitalIn`, `AnalogIn`, sensors, etc.): `📥 Place InputRef on Page`
    - For Outputs (`VirtualOut`, `VirtualOutCmd`, `DigitalOut`, `AnalogOut`, actuators, etc.): `📥 Place OutputRef on Page` (plus `📥 Place InputRef` if readable).
- **Disconnect Wire Buttons in Inspector:**
  - In the **Inputs & Outputs (`<Co>`)** section of `PageBlock` in `inspector.rs`, each incoming wire (`←`) and outgoing wire (`→`) row gains a compact `✕` button that disconnects that wire (`ProjectEdit::RemoveWireLink`).
- **Delete Object Button in Inspector:**
  - `PageBlock`, `Page` (when >1 page exists), `Place`, `Category`, and user-creatable `TreeNode` (`Merker`, `VirtualIn`, `VirtualOut`, `VirtualOutCmd`) show a `🗑 Delete` button in the Inspector header that emits the corresponding `Remove*` `ProjectEdit` and clears selection.

### 4.3 Undo / Redo Stack & Top Bar (`app.rs`)
- `LoxoneApp` maintains `undo_stack: Vec<EditRecord>` and `redo_stack: Vec<EditRecord>` where `EditRecord { forward: ProjectEdit, inverse: ProjectEdit }`.
- Executing a new edit pushes to `undo_stack`, clears `redo_stack`, marks `is_dirty = true`, and rebuilds `xref`.
- `Cmd+Z` / `Ctrl+Z` (or clicking `↩ Undo` in the top bar) pops `undo_stack`, applies `record.inverse`, and pushes to `redo_stack`.
- `Cmd+Shift+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` (or clicking `↪ Redo` in the top bar) pops `redo_stack`, applies `record.forward`, and pushes to `undo_stack`.

---

## 5. Testing & Verification Strategy

1. **Catalog & Factory Unit Tests (`crates/loxone-core`):**
   - Verify all 76 `BlockSchema` entries have valid non-empty type names, categories, descriptions, and pin definitions.
   - Verify `instantiate_block_from_schema` for multiple block types (`LightControllerV2`, `Pushbutton`, `PID`, `ThresholdSwitch`, etc.) produces valid `PageBlock`s with expected `<Co>` pins and default attributes.
   - Verify `instantiate_periphery_ref` and `instantiate_memory_bar` produce valid `InputRef`, `OutputRef`, and `Memory` blocks with `Ref` attributes and `<Co>` pins.
2. **Structural `ProjectEdit`, Undo/Redo & XML Round-Trip Tests (`crates/loxone-core`):**
   - Test adding a block, moving it, wiring two blocks together (`AddWireLink`), saving to XML, and re-parsing to confirm `<C>` blocks and `<Co Nc="1"><In Input="..."/></Co>` wires survive round-trip in `Klimczaki.Loxone`.
   - Test `RemoveWireLink` and `RemovePageBlock` (including automatic cleanup of connected wires) and verify applying the returned inverse `ProjectEdit` restores the exact pre-edit state.
   - Test `AddPage`/`RemovePage`, `AddRoom`/`RemoveRoom`, `AddCategory`/`RemoveCategory`, and `AddTreeNode`/`RemoveTreeNode` round-trip in XML.
3. **GUI Integration Tests (`crates/loxone-gui`):**
   - Test Quick-Insert modal filtering, canvas wire hit-testing, and Undo/Redo state transitions in `LoxoneApp`.
   - Ensure `cargo test --workspace` passes with 0 warnings.
