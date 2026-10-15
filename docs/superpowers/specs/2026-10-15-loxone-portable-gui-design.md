# Loxone Portable GUI — Phase 1 Design Specification

**Date:** 2026-10-15  
**Project:** `loxone-portable`  
**Scope:** Phase 1 — Cross-Platform Rust Loxone Config Project Viewer, Visual Function-Block Canvas & Cross-Reference Navigator

---

## 1. Purpose & Goals

Official Loxone Config is Windows-only. The goal of `loxone-portable` is to build a native, cross-platform Rust GUI application for inspecting, navigating, and porting Loxone Config projects (`.Loxone` files).

**Phase 1 Milestone** delivers a complete read-only Loxone Config project workbench validated against real-world `.Loxone` files (specifically `Klimczaki.Loxone`, ControlList v274, 1,493 `<C>` controls, 28 Pages, 16 Rooms, 33 Categories, 5,433 total `<Co>` pins — 4,336 on Page blocks and 1,097 on Periphery/System nodes):
1. Lossless `.Loxone` XML parser (`<ControlList>`) supporting UTF-8 BOM, hierarchical periphery/system trees, pages, function blocks, pins, wires, and raw XML metadata.
2. Precomputed UUID & Cross-Reference index linking Peripherals, Markers, Modes, Rooms, Categories, Function Blocks, and Connector Pins across all pages.
3. Clean, minimalist 3-pane workspace built with pure-Rust `eframe` / `egui` (inspired by Linear / Figma / Raycast rather than legacy Windows ribbon clutter):
   - **Minimalist Top Bar:** Subtle breadcrumb, `Ctrl+K` / `⌘K` Command Palette trigger pill, sidebar toggles, and compact zoom indicator.
   - **Collapsible Left Explorer (`Ctrl+B`):** Clean, border-subtle Project & Periphery tree (`Pages`, `Periphery`, `Rooms`, `Categories`, `System`).
   - **Center 2D Schematic Canvas:** Matte dark canvas (`#111317`) with subtle dot grid, tasteful desaturated `Cl` block header accents, option to collapse unconnected pins for visual clarity, and smooth wire curves.
   - **Collapsible Right Inspector (`Ctrl+I`):** Minimalist property cards, clickable **"Used On Pages"** cross-reference pills, pin/parameter (`Def`) table, and on-demand raw XML drawer.
4. **Keyboard-First `Ctrl+K` / `Cmd+K` Command Palette:**
   - Instant modal launcher for all operations (open file, fit canvas, toggle panels, toggle compact pins) and instant fuzzy/substring navigation to any Page, Function Block, Peripheral, Room, Category, or UUID with `↑`/`↓`/`Enter`/`Esc`.

---

## 2. Workspace Architecture

```text
loxone-portable/
├── Cargo.toml                  # Cargo workspace root
├── Klimczaki.Loxone            # Real-world reference Loxone Config project
├── crates/
│   ├── loxone-core/            # Pure Rust domain model, XML parser, and cross-reference engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs       # quick-xml reader -> RawControl & LoxoneProject
│   │       ├── model.rs        # Typed domain structs (Page, PageBlock, ConnectorPin, Periphery, etc.)
│   │       └── xref.rs         # UUID lookup table & cross-reference index
│   └── loxone-gui/             # Pure Rust eframe/egui desktop application
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # App entry point, CLI arg / auto-load Klimczaki.Loxone
│           ├── app.rs              # Top-level AppState, keyboard shortcuts, navigation controller
│           ├── theme.rs            # Minimalist dark theme, typography, spacing, desaturated Cl colors
│           ├── command_palette.rs  # Ctrl+K / Cmd+K modal for operations & instant object navigation
│           ├── tree_panel.rs       # Left panel: Periphery, Pages, Rooms, Categories, System tree
│           ├── canvas.rs           # Center panel: 2D function-block & wire schematic renderer
│           └── inspector.rs        # Right panel: Properties, Pins/Parameters, Cross-References, Raw XML
```

---

## 3. Domain Data Model & Parser (`loxone-core`)

### 3.1 Lossless Raw Representation (`RawControl`)
Every `<C>` element and its non-`<C>` children (`<Co>`, `<In>`, `<IoData>`, `<Display>`, `<LightscenesC>`, `<LoggerMailer>`, etc.) are parsed via `quick-xml`:
- `RawNode`: stores tag name, ordered attributes `BTreeMap<String, String>` (plus original insertion order list for attribute display), text content, and child `Vec<RawNode>`.
- Handles UTF-8 BOM (`\u{FEFF}`) transparently and preserves unknown block types or vendor sub-elements without failing.

### 3.2 Typed Project Model (`LoxoneProject`)
Built from the `<ControlList>` and root `<C Type="Document">` element:
- **`ProjectMetadata`**: `title`, `uuid`, `config_version`, `control_list_version`, `creator`, `street`, `town`, `postcode`, `country`, `latitude`, `longitude`, `currency`, `app_id`.
- **`categories: IndexMap<String, Category>`**: Parsed from `<C Type="CategoryCaption">` children (`U`, `Title`, `CatGroup`, `BkColor`, `Icon`).
- **`places: IndexMap<String, Place>`**: Parsed from `<C Type="PlaceCaption">` children (`U`, `Title`, `Sqm`, `PType`, `Icon`).
- **`pages: Vec<Page>`**: Parsed from `<C Type="Program">` -> `<C Type="Page">`:
  - `uuid: String`, `title: String`, `room_uuid: Option<String>`, `category_uuid: Option<String>`, `blocks: Vec<PageBlock>`.
- **`PageBlock`**:
  - `uuid: String`, `block_type: String`, `title: String`
  - `px: f32`, `py: f32`, `px2: f32`, `py2: f32` (parsed from `Px`, `Py`, `Px2`, `Py2`; defaults to `0.0` if absent)
  - `color_rgb: Option<[u8; 3]>` (parsed from `Cl="r,g,b"`)
  - `ref_uuid: Option<String>` (from `Ref="..."`, or `RefCaller` on `<LoggerMailer>`)
  - `extra_ref_uuids: Vec<String>` (comma-separated UUIDs in `UO="..."` or `SpStates="..."`)
  - `room_uuid: Option<String>`, `category_uuid: Option<String>` (from child `<IoData Pr="..." Cr="..."/>`)
  - `text_content: Option<String>` (from `Text="..."` attribute on `Type="Text"` blocks)
  - `pins: Vec<ConnectorPin>` (from child `<Co>` elements)
  - `raw_attributes: Vec<(String, String)>` and `extra_children: Vec<RawNode>`
- **`ConnectorPin`**:
  - `uuid: String` (`U`), `key: String` (`K`), `default_value: Option<String>` (`Def`), `pin_kind: PinKind` (`Input`, `Parameter`, `Output`), `incoming_links: Vec<WireLink>` (from `<In Input="..." FLG="..."/>`).
  - **Pin Classification Rule (`PinKind`):**
    1. If the `<Co>` has `Def="..."` and no incoming `<In>` wires, it is classified as `PinKind::Parameter`.
    2. If the `<Co>` is referenced as a wire source by any `<In Input="...">` in the project, OR its key `K` matches Loxone output naming conventions (`Q`, `Q*`, `AQ`, `AQ*`, `Out*`, `O*` on meter/EFM blocks, `TQ*` when after inputs and without `Def`, `TxQ*`, `Scene`, `RoomOff`, `HouseOff`, `TargetPos`, `Light`, `Pulse`, `Rest`, `API`, `OutputAPI`, `Level1`..`Level10`, `ActiveLevel`, `LastStart`, `LastStop`, `ConfirmCause`, `ConfirmTime`), it is classified as `PinKind::Output`.
    3. Otherwise (or if it has `<In>` child elements), it is classified as `PinKind::Input`.
- **`TreeSection` & `TreeNode`**:
  - Organizes the non-Page hierarchy of `<C Type="Document">` into navigable tree sections:
    - `Miniserver & Periphery` (`LoxLIVE` subtree: Digital/Analog Inputs/Outputs, Virtual I/O, Loxone Tree `LoxTree` -> `TreeDevice` -> sensors/actors, Loxone Air `LoxAIRDevice` -> sensors/actors, Intercom, Weather, Messages).
    - `System & Logic` (`ModeCaption`, `MemoryCaption`, `GlobalStates`, `TimeCaption`, `UserCaption`, `WeatherServer`, `AutoPilot`, etc.).

### 3.3 Cross-Reference Index (`XrefIndex`)
Computed once when `LoxoneProject` is constructed:
- `objects_by_uuid: HashMap<String, ObjectSummary>` — maps every `<C U="...">` UUID in the document (tree nodes, peripherals, rooms, categories, pages, page blocks) to its `ObjectSummary { uuid, title, object_type, location, room_uuid, category_uuid }`.
- `pins_by_uuid: HashMap<String, PinLocation>` — maps every `<Co U="...">` UUID across the entire document (both page blocks and peripheral/tree `<C>` nodes that own output `<Co>` pins) to `(OwnerUuid, Option<PageUuid>, PinKey)`.
- `usages_by_target_uuid: HashMap<String, Vec<CrossRefUsage>>` — for any target object UUID (e.g., a `TreeSensor`, `DigitalIn`, `Actor`, `Caller`, `Memory`, or `PageBlock`), records every `PageBlock` that references it via:
  1. Direct `Ref="..."` (`InputRef`, `OutputRef`, `OutputRefLM`)
  2. `<LoggerMailer RefCaller="..."/>`
  3. `UO="..."` comma-separated device references (e.g., on `AalEmergency`, `AalSmartAlarm`)
  4. `<In Input="pin_uuid"/>` where `pin_uuid` belongs to that target object.

---

## 4. Visual Page Canvas (`loxone-gui::canvas`)

### 4.1 Coordinate Transform & Camera
- Logical Loxone coordinates `(Px, Py)` are transformed to screen coordinates via:
  $$\text{screen\_pos} = \text{canvas\_rect.min} + \text{pan} + \text{loxone\_pos} \times \text{zoom}$$
- Default `zoom = 0.085` (`192` Loxone units $\approx 16.3\text{ px}$).
- `fit_to_page(page, canvas_size)` computes the bounding box of all `blocks` on the active `Page` with `40px` padding and sets `zoom` (clamped to `[0.02, 0.35]`) and `pan` so the entire page diagram is centered on screen.
- Interactions:
  - Scroll wheel / trackpad pinch zooms smoothly around the cursor position.
  - Dragging empty canvas background or middle-mouse dragging pans the viewport.
  - Clicking a block selects it and updates the Right Inspector; double-clicking an `InputRef`/`OutputRef` or cross-reference badge navigates to its target or opens a quick jump popup.

### 4.2 Block Rendering Styles
1. **Reference Bars (`InputRef`, `OutputRef`, `OutputRefLM`, `Memory`):**
   - Rendered within `(Px, Py)..(Px2, Py2)` with a subtle border, category/room tint accent on the left strip, and bold `Title` (falling back to the resolved `Ref` target's `Title` if empty).
   - Displays input connector dot on the left (`AI`/`I`) and output connector dot on the right (`AQ`/`Q`).
2. **Function Blocks (all other `<C>` with `<Co>` pins):**
   - Outer rounded rectangle `(Px, Py)..(Px2, Py2)` with dark card background (`#1B1F2A`) and selection glow (`#5E81FF`) when active.
   - Top header band (`22px` scaled height) filled with the block's desaturated `Cl` RGB color + crisp `3px` full-saturation left accent strip, displaying `Title` and `Type` subtitle.
   - **Left column:** Visible `Input` pins (`K` label + pin anchor dot on the left border). If an input pin has external/hidden links (`FLG="1"` or off-page source), renders a compact pill badge `🔗 <Source Title> (×N)` to the left of the pin.
   - **Right column:** `Output` pins (`K` label right-aligned + pin anchor dot on the right border).
3. **Notes (`Type="Text"`):**
   - Warm note rectangle tinted by `Cl` (`255,200,100`), header `Title`, and word-wrapped `Text` body.

### 4.3 Wire Routing
- Every `<In Input="src_pin_uuid">` on the active page is checked against `pins_by_uuid`:
  - **Same-page wire:** If `src_pin_uuid` belongs to a block on the active page, draw an orthogonal 3-segment (or cubic bezier) path from `src_pin_anchor` (right edge of source block) to `dst_pin_anchor` (left edge of target block).
  - **Wire styling:** Analog pins (`AQ*`/`AI*`) use warm amber/orange strokes; digital pins (`Q*`/`I*`) use crisp cyan/green strokes; wires connected to the currently selected block are drawn thicker (`2.5px`) and brighter, while unrelated wires dim slightly when a block is selected.

---

## 5. Minimalist UI, `Ctrl+K` Command Palette & Cross-Reference Navigation (`loxone-gui`)

### 5.1 Minimalist Visual Language (`theme.rs`)
- **Color Palette:**
  - Deep matte canvas background: `#111317` with subtle grid dots (`#1E222B`).
  - Side panels & top bar: `#161920` separated by hairline `1px` borders (`#242936`).
  - Cards & nodes on canvas: `#1B1F2A` fill, `#2C3344` border (`#5E81FF` glow when selected).
  - Block header `Cl` colors are blended (`35%` Loxone RGB + `65%` dark surface) with a crisp `3px` left accent bar at full saturation so neon Loxone colors (`255,0,84`, `164,112,255`) look refined and legible.
- **Decluttered Canvas Mode:**
  - By default (toggleable via `Ctrl+K` -> *Toggle Unconnected Pins* or toolbar switch), function blocks on the canvas can either show all pins or prioritize connected pins + compact summary count (`+14 unconnected`) to keep diagrams clean and readable.

### 5.2 `Ctrl+K` / `Cmd+K` Command Palette (`command_palette.rs`)
- Pressing **`Ctrl+K`** (or **`Cmd+K`** on macOS) anywhere in the app opens a centered, floating command palette modal (`540px` wide) with auto-focused search input:
  - **Keyboard Navigation:** `↑` / `↓` (or `Ctrl+N` / `Ctrl+P`) to move highlight, `Enter` to execute, `Esc` to close.
  - **Unified Search & Actions:**
    1. **Operations (`Action` badge):**
       - `Open .Loxone Project File…` (`Ctrl+O`)
       - `Fit Active Page to View` (`F`)
       - `Reset Canvas Zoom (100%)` (`0`)
       - `Toggle Compact Pins (Hide/Show Unconnected Pins)` (`P`)
       - `Toggle Left Explorer Panel` (`Ctrl+B`)
       - `Toggle Right Inspector Panel` (`Ctrl+I`)
    2. **Pages (`Page` badge):** Instant jump to any of the 28 pages (e.g., typing `alarm` or `audio` opens that page and fits the camera).
    3. **Function Blocks (`Block` badge):** Searches across all 1,493 objects by Title, Type (`LightController2`, `AutoJalousie`), Room, or UUID; pressing `Enter` opens the block's page, selects the block, and centers the canvas on it.
    4. **Peripherals, Rooms & Categories (`Periphery` / `Room` / `Category` badge):** Selects the item in the explorer and opens its properties & **"Used On Pages"** cross-references in the Right Inspector.

### 5.3 Minimalist Top Bar
- **Left:** Sidebar toggle icon (`Ctrl+B`), Project Title & active Page breadcrumb (`Klimczaki  /  Alarm 1`).
- **Center:** Prominent clickable **`Search or run command…  Ctrl+K`** pill button that opens the Command Palette.
- **Right:** Compact pin-mode toggle (`Connected Pins` / `All Pins`), zoom percentage pill (`Fit`, `-`, `100%`, `+`), and Right Inspector toggle (`Ctrl+I`).

### 5.4 Left Panel — Explorer Tree (`tree_panel.rs`)
- Clean, single-column segmented switcher at top (`Pages` | `Periphery` | `Rooms` | `System`) so the user isn't overwhelmed by deeply nested root folders:
  1. **Pages (`28`):** Clean list of pages with subtle block-count pill. Clicking opens the page.
  2. **Periphery (`LoxLIVE`):** Hierarchical tree of Miniserver I/O, Loxone Tree devices, Loxone Air devices, Virtual I/O, Intercom, Weather, and Messages, with subtle `[N]` usage count badges.
  3. **Rooms (`16`) & Categories (`33`):** Clean collapsible groups showing objects assigned to each Room (`Pr`) or Category (`Cr`).
  4. **System:** Modes, Markers/Memories, System Variables, Users, Timers.

### 5.5 Right Panel — Inspector & Cross-References (`inspector.rs`)
- Displays clean, card-based details for `selected_uuid`:
  1. **Identity Header:** `Title`, subtle `Type` pill, copyable `UUID` button, and resolved `Room` & `Category` tags.
  2. **Target Link (if `Ref` is present):** Shows the referenced peripheral/marker with a one-click **"Inspect Target"** button.
  3. **Cross-References ("Used On Pages"):** Clean list of clickable rows (`Page Title  →  Block Title`). Clicking any row jumps straight to that Page and centers the block on the canvas.
  4. **Pins & Parameters (`<Co>`):** Minimalist grouped list of Parameters (`Def`), Connected Inputs/Outputs (clickable to jump to the other end of the wire), and a collapsible section for unconnected pins.
  5. **Raw XML Drawer:** Collapsed by default at the bottom so the inspector stays clean, expandable with one click when inspecting low-level Loxone attributes.

---

## 6. Error Handling & Testing Strategy

- **Error Handling:**
  - File I/O and XML syntax errors return structured `Result<LoxoneProject, LoxoneError>` (`thiserror`) and are displayed in a non-crashing modal/banner in the GUI.
  - Missing coordinates or malformed `Cl` color strings fall back gracefully to default dimensions/colors.
- **Automated Verification (`cargo test`):**
  - **Unit tests** for `Cl` color parsing, `PinKind` classification, command palette fuzzy/substring matching, and XML snippet parsing.
  - **Integration tests against `Klimczaki.Loxone`** verifying:
    - Document title `"Klimczaki"` and version `"17020828"` parse accurately.
    - Exactly `28` pages, `16` places/rooms, and `33` categories are extracted.
    - All `1,493` `<C>` elements and `5,433` `<Co>` pins (`4,336` on pages + `1,097` on periphery/system nodes) are indexed in `XrefIndex`.
    - Both same-page wires (`459` source pins) and cross-page/peripheral hidden wires (`216` external source pins) resolve in `XrefIndex`.
    - Cross-reference lookup for peripherals referenced by `InputRef`/`OutputRef` returns non-empty page usages.
