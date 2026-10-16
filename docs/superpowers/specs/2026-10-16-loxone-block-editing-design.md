# Loxone Config–Style Block & Periphery Attribute Editing Design

**Date:** 2026-10-16
**Status:** Approved
**Target Workspace:** `/Users/bartlomiej.klimczak/Projects/loxone-portable`

---

## 1. Overview & Goals

Enable Loxone Config–style editing of block attributes, parameter pins, child configuration tags, and periphery/system object properties in `loxone-portable`, backed by official Loxone Knowledge Base (`loxone.com/enen/kb/`) documentation for every block and object type present in `Klimczaki.Loxone`.

### Key Requirements
1. **Official Loxone KB Schema Registry (`crates/loxone-core/src/block_schema.rs`)**:
   - Comprehensive, zero-GUI-dependency static metadata covering all **41 page-level `<C Type="...">` function/reference block types** and all **periphery, task, and system `<C Type="...">` object types** present in `Klimczaki.Loxone`.
   - Each schema entry defines:
     - Official Loxone Config display name, category group, Loxone KB documentation URL (`https://www.loxone.com/enen/kb/...`), and functional summary.
     - Documented `<C>` attributes (`Bool`, `Int`, `Float`, `Enum`, `String`, `ColorRgb`, `UuidRef`) with labels, units, enum option tables, and KB explanations.
     - Documented `<Co K="..." Def="..."/>` parameter pins (`Float`, `Int`, `Bool`, `Enum`, `String`) with human-readable labels, units (`s`, `min`, `°C`, `K`, `%`, `°`, `lx`, `kW`, `kWh`), min/max ranges, default values, enum options, and KB explanations.
     - Official Loxone KB full names and descriptions for all Input (`I*`, `In*`) and Output (`Q*`, `AQ*`, `Out*`) connector pins.
     - Supported child XML tags (`<IoData>`, `<Display>`, `<LoggerMailer>`, `<HCL>`, `<PSD>`, `<COHist>`).
2. **Lossless XML Mutation & File Writer (`crates/loxone-core/src/writer.rs`)**:
   - Mutates in-memory `LoxoneProject` structures (`PageBlock`, `TreeNode`, `Place`, `Category`, `XrefIndex`) and writes changes back into the `.Loxone` XML document while preserving UTF-8 BOM (`\u{FEFF}`), CRLF (`\r\n`) line endings, attribute ordering, and all untouched XML nodes (`<LightscenesC>`, `<LSConfig>`, `<Timer>`, `<Entry>`, `<statGrp>`, etc.).
   - Supports atomic save to disk (`save_loxone_file`) and round-trip serialization (`serialize_loxone_xml`).
3. **Loxone Config–Style Property Inspector (`crates/loxone-gui/src/inspector.rs`)**:
   - **Header & KB Documentation Banner**: Official block name, XML `Type` badge, KB summary, and clickable `📖 Loxone Docs` button (`ctx.open_url`).
   - **General & Visualization**: Editable `Title`, `Desc`, Room (`IoData.Pr`) dropdown, Category (`IoData.Cr`) dropdown, `Visu`, `VisuPwd`, `UseInFav`, `Rating`, block color (`Cl`) swatches/RGB, `<Display>` (`Unit`, `StateOnly`), and `Text` note content.
   - **KB-Documented Parameters & Modes**: Native controls (dropdowns for enums, checkboxes for booleans, numeric fields with units/ranges, text inputs) with hover tooltips explaining each parameter and its XML/pin key.
   - **Inline Linked Periphery/System Target Editor**: Selecting an `InputRef`, `OutputRef`, or `OutputRefLM` block on a schematic page displays an inline editor for its linked `Ref` / `RefCaller` / `RefNotification` target (`TreeSensor`, `TreeAsensor`, `TreeActor`, `TreeAactor`, `Actor`, `DigitalIn`, `Dimmer`, `VirtualOutCmd`, `Caller`, `Notification`, `Memory`, `Mode`, `SysVar`) right inside the Inspector.
   - **Child Configuration Editors**: Structured editors for `<LoggerMailer>`, `<HCL>`, and `<PSD>` when supported/present.
   - **Documented Inputs & Outputs + Collapsible Advanced Editor**: Shows official Loxone KB names and descriptions next to each connector pin and provides a collapsible Advanced section to edit any raw `<Co Def="..."/>` or `<C>` attribute.
4. **Top Action Bar Save & Dirty State (`crates/loxone-gui/src/app.rs`)**:
   - Tracks `is_dirty: bool` when any edit occurs, shows an unsaved `*` indicator in the toolbar and status bar, and provides a `💾 Save` button (`Cmd+S` / `Ctrl+S`) in the top action bar.

---

## 2. Architecture & Module Structure

```
crates/
├── loxone-core/
│   └── src/
│       ├── lib.rs            # Exports block_schema and writer modules
│       ├── model.rs          # Mutability helpers on PageBlock, TreeNode, ConnectorPin, RawNode
│       ├── parser.rs         # Existing lossless XML parser
│       ├── xref.rs           # Existing cross-reference index (rebuilt on title/ref edits)
│       ├── block_schema.rs   # NEW: Official Loxone KB schema registry for all Klimczaki.Loxone types
│       └── writer.rs         # NEW: In-memory project mutator + lossless XML serializer & disk writer
└── loxone-gui/
    └── src/
        ├── app.rs            # Dirty state tracking, Save (Cmd+S / Ctrl+S) button, mutation dispatch
        ├── inspector.rs      # Loxone Config–style property editor & KB documentation viewer
        ├── canvas.rs         # Existing schematic canvas
        ├── tree_panel.rs     # Existing project tree
        ├── command_palette.rs# Existing Ctrl+K palette
        └── theme.rs          # Existing dark theme
```

---

## 3. Data Model & Schema Registry (`crates/loxone-core/src/block_schema.rs`)

### 3.1 Core Schema Types

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueKind {
    Bool,
    Int { min: Option<i64>, max: Option<i64> },
    Float { min: Option<f64>, max: Option<f64>, step: f64 },
    Enum(&'static [(&'static str, &'static str)]), // (raw_value, label)
    String,
    MultilineString,
    ColorRgb,
    UuidRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildConfigKind {
    IoData,
    Display,
    LoggerMailer,
    Hcl,
    Psd,
    CoHist,
}

#[derive(Debug, Clone, Copy)]
pub struct AttributeSchema {
    pub key: &'static str,
    pub label: &'static str,
    pub unit: Option<&'static str>,
    pub kind: ValueKind,
    pub default_value: Option<&'static str>,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct PinParamSchema {
    pub key: &'static str,
    pub label: &'static str,
    pub unit: Option<&'static str>,
    pub kind: ValueKind,
    pub default_value: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct PinDoc {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct BlockSchema {
    pub type_name: &'static str,
    pub display_name: &'static str,
    pub category_group: &'static str,
    pub kb_url: Option<&'static str>,
    pub summary: &'static str,
    pub attributes: &'static [AttributeSchema],
    pub parameters: &'static [PinParamSchema],
    pub pin_docs: &'static [PinDoc],
    pub child_configs: &'static [ChildConfigKind],
}
```

### 3.2 Complete Coverage of `Klimczaki.Loxone` Types

1. **41 Page-Level Block Types**:
   - **Lighting**: `LightController2`, `CentralLight`
   - **Shading & Doors**: `AutoJalousie`, `CentralShade`, `Door`, `Doorcontroller`
   - **Climate & HVAC**: `HeatIRoomController2`, `ClimateControllerUS`, `HVACController`, `Heatmixer2`
   - **Audio**: `MusicPlayer`, `CentralMusic`
   - **Access & Intercom**: `Intercom`, `NfcCodeTouch`
   - **Safety, Security & AAL**: `Alarm`, `SmokeAlarm`, `AlarmChain`, `AalEmergency`, `AalSmartAlarm`, `PresenceDetector`
   - **Energy & Meters**: `EFM`, `MeterAbsBi`, `MeterPUni`
   - **Switches & Clicks**: `PushButton`, `PButtonT`, `MultiClick`, `LongClick`
   - **Timers & Pulses**: `AlarmClock`, `OnPulseDelay`, `PulseAt`, `PulseBy`
   - **Logic & Comparison**: `And`, `Not`, `Equal`
   - **Status, Memory & Text**: `State`, `StateV`, `Memory`, `Text`
   - **Page I/O Reference Blocks**: `InputRef`, `OutputRef`, `OutputRefLM`
2. **Tree / Periphery / Task / System Object Types**:
   - **Tasks & Schedules**: `SwitchingTimer`, `CalendarEntry`, `PuDe`
   - **Miniserver & Extension I/O**: `DigitalIn`, `Actor`, `Dimmer`, `VoltageIn`, `LoxDIMM`, `LoxREL`, `LoxTree`, `LoxAIR`, `Comm485`
   - **Loxone Tree & Air Devices & I/O**: `TreeDevice`, `TreeSensor`, `TreeAsensor`, `TreeActor`, `TreeAactor`, `TreeTextActor`, `LoxAIRDevice`, `LoxAIRsensor`, `LoxAIRAsensor`, `LoxAIRactor`, `LoxAIRAactor`
   - **Modbus, Virtual I/O & Network/Audio**: `ModbusDev`, `ModbusASensor`, `VirtualOut`, `VirtualOutCmd`, `AudioServer`, `AudioOut`, `IntercomDevice`, `NetworkDevice`, `MTablet`, `GenSensor`, `GenAsensor`, `ApiActor`, `Plugin`
   - **Messages, Logging & Notifications**: `Caller`, `CallerVirtualIn`, `Mailer`, `Logger`, `Notification`
   - **System, Operating Modes & Weather**: `Mode`, `SysVar`, `Constant`, `WeatherServer`, `WeatherData`, `Place`, `Category`, `User`, `UserGroup`, `Online`, `OvertempShutdown`, `SysTemp`, and time/astronomy nodes (`Sunrise`, `Sunset`, `ImpulseSunrise`, `ImpulseSunset`, `NightTime`, `Time`, `DateTime`, `Day`, `Month`, `Year`, etc.).

---

## 4. Mutation & Lossless XML Writer (`crates/loxone-core/src/writer.rs`)

### 4.1 Edit Operations (`ProjectEdit`)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectEdit {
    /// Set or remove an attribute on `<C U="target_uuid">`
    SetControlAttr {
        target_uuid: String,
        key: String,
        value: Option<String>,
    },
    /// Set or remove `Def` on `<Co K="pin_key">` inside `<C U="target_uuid">`
    /// (creates `<Co K="pin_key" Def="..."/>` if the parameter pin does not yet exist on the block)
    SetPinDefault {
        target_uuid: String,
        pin_key: String,
        default_value: Option<String>,
    },
    /// Set or remove an attribute on a child config tag (`IoData`, `Display`, `LoggerMailer`, `HCL`, `PSD`, `COHist`)
    /// inside `<C U="target_uuid">` (creates the child tag if it does not exist yet and `value` is `Some`)
    SetChildTagAttr {
        target_uuid: String,
        child_tag: String,
        key: String,
        value: Option<String>,
    },
}
```

### 4.2 Synchronizing In-Memory Model & `raw_xml`
When `apply_edit(project: &mut LoxoneProject, edit: &ProjectEdit)` is called:
1. Updates the corresponding `PageBlock` (on `project.pages`) and/or `TreeNode` (on `project.periphery_tree` / `project.system_tree`), plus `project.places` / `project.categories` if a `Place` or `Category` title/color is edited:
   - Synchronizes derived fields (`title`, `color_rgb`, `text_content`, `room_uuid`, `category_uuid`, `ref_uuid`, `pins[..].default_value`, `raw_attributes`, `extra_children`).
   - Rebuilds `project.xref = XrefIndex::build(project)` whenever `Title`, `Pr`, `Cr`, `Ref`, or `RefCaller` changes so cross-references, room/category badges, and search palette stay immediately up to date.
2. When `serialize_loxone_xml(project: &LoxoneProject)` or `save_loxone_file(path: &Path, project: &LoxoneProject)` is invoked:
   - Parses `project.raw_xml` into the lossless `RawNode` tree (or mutates the `RawNode` tree directly), applies all current block/tree node attributes, `<Co>` pins, and `extra_children` by matching `<C U="...">`, and serializes back to XML with:
     - Original UTF-8 BOM (`\u{FEFF}`) if present in `raw_xml` (defaulting to `\u{FEFF}<?xml version="1.0" encoding="utf-8"?>\r\n`),
     - CRLF (`\r\n`) line endings matching Loxone Config,
     - Proper XML escaping of attribute values (`&quot;`, `&lt;`, `&gt;`, `&amp;`),
     - Updates `project.raw_xml` with the newly serialized XML string.

---

## 5. Inspector UI Design (`crates/loxone-gui/src/inspector.rs`)

### 5.1 Return Signature
`render_inspector` returns an `InspectorOutput`:
```rust
#[derive(Default)]
pub struct InspectorOutput {
    pub nav_target: Option<InspectorNavTarget>,
    pub edits: Vec<ProjectEdit>,
}
```

### 5.2 Inspector Sections
1. **Header & Loxone KB Banner**:
   - Displays `schema.display_name` (e.g. `"Automatic Shading"`) and `block_type` badge (`AutoJalousie`).
   - Shows `schema.summary` in muted text.
   - If `schema.kb_url` is `Some(url)`, renders a `📖 Loxone Docs` button that calls `ui.ctx().open_url(egui::OpenUrl::new_tab(url))`.
2. **General & Visualization (`egui::CollapsingHeader::new("General & Visualization").default_open(true)`)**:
   - **Title** (`Title`) text edit & **Description** (`Desc`) text edit.
   - **Room** (`IoData.Pr`) `ComboBox` listing `<None>` and all `project.places`.
   - **Category** (`IoData.Cr`) `ComboBox` listing `<None>` and all `project.categories`.
   - **Visualization Flags** (`IoData`):
     - `Use in User Interface` (`Visu="true"`)
     - `Password Protected` (`VisuPwd="true"`)
     - `Favorite` (`UseInFav="true"`)
     - `Importance / Rating` (`Rating`) `DragValue` (`0..=10`)
   - **Color (`Cl`)**: Loxone color preset buttons (`Green`, `Yellow`, `Red/Alarm`, `Purple/Audio`, `Blue`, `Gray`) + RGB editor.
   - **Display Format (`<Display>`)**: When `ChildConfigKind::Display` is supported or `<Display>` exists, edits `Unit` (e.g. `<v.1>°`) and `StateOnly`.
   - **Comment Text (`Text`)**: Multiline text editor when `block_type == "Text"`.
3. **Parameters & Operating Modes (`egui::CollapsingHeader::new("Parameters & Operating Modes").default_open(true)`)**:
   - Iterates through `schema.attributes` (block-specific `<C>` attributes) and `schema.parameters` (`<Co K="..." Def="..."/>` parameter pins):
     - Renders label + unit badge + hover tooltip with `description` and XML key (`K="..."` or `Attr="..."`).
     - Renders `ValueKind::Bool` as checkbox, `ValueKind::Enum` as `ComboBox`, `ValueKind::Int` / `ValueKind::Float` as numeric `DragValue` / text input with unit suffix, and `ValueKind::String` / `MultilineString` as text edit.
     - Shows a subtle `"reset"` button when a parameter overrides `schema.default_value`.
4. **Child Configuration Sections (when applicable)**:
   - **`<LoggerMailer>`** (on `OutputRefLM`): `On` text template, `Off` text template, `MinimumTime` (ms), target `RefCaller` / `RefNotification`.
   - **`<HCL>`** (on `LightController2`): Daylight color temperature curve (`TMIN`, `TMAX`, `FROM`, `TO`, `M`, `T`).
   - **`<PSD>`** (on `LightController2`, `MusicPlayer`, `PushButton`): Presence simulation settings (`En`, `Dmin`, `Dmax`, `Amin`, `Amax`, `V`).
5. **Linked Periphery / System Target Section (for `InputRef`, `OutputRef`, `OutputRefLM`, `AutoJalousie`, `NfcCodeTouch`, `Doorcontroller`, `Intercom`)**:
   - When a page block has `ref_uuid = Some(target_uuid)` pointing to a `TreeNode` (or `Memory` block), renders an inline collapsible section **`Linked Target: <Title> (<Type>)`** with a `"Jump to Tree"` button **and** directly embeds the General + Parameters editor for that target `TreeNode` (`TreeSensor`, `TreeAsensor`, `TreeActor`, `TreeAactor`, `Actor`, `DigitalIn`, `Dimmer`, `VirtualOutCmd`, `Caller`, `Notification`, `Mode`, `SysVar`, etc.).
6. **Inputs & Outputs (with Official KB Pin Docs)**:
   - Each input/output pin displays `PinDoc.label` (e.g. `"Td — Disarm Delay"` instead of just `WI2`) and shows `PinDoc.description` on hover, while preserving clickable wire navigation links.
7. **Collapsible Advanced XML Editor (`default_open(false)`)**:
   - Allows editing `Def` on any `<Co>` pin and editing/adding any raw `<C>` attribute not already covered above.

---

## 6. Testing & Verification Strategy

1. **Schema Coverage Unit Test (`crates/loxone-core/src/block_schema.rs`)**:
   - Loads `Klimczaki.Loxone` and asserts that **every single `<C Type="...">`** on every page and in the periphery/system trees resolves to a registered `BlockSchema` with non-empty `display_name` and `summary`.
2. **Lossless XML Writer & Round-Trip Tests (`crates/loxone-core/src/writer.rs`)**:
   - Verifies `ProjectEdit::SetControlAttr`, `ProjectEdit::SetPinDefault`, and `ProjectEdit::SetChildTagAttr` update both the in-memory `LoxoneProject` and serialized XML.
   - Verifies round-trip parsing of serialized `Klimczaki.Loxone` preserves all 28 pages, 16 rooms, 33 categories, Miniserver metadata, UTF-8 BOM, and modified values.
3. **Inspector & App Integration Tests (`crates/loxone-gui`)**:
   - Verifies applying edits marks `app.is_dirty = true`, updates `XrefIndex`, and saving writes cleanly to disk and clears `is_dirty`.
