# Loxone Portable

The application is a Loxone Config alternative that's written in Rust and fully compatible with Mac/Windows/Linux.

I often use non-windows OSs and it forced me to use a VM in order to edit anything in Loxone Config. This project solves the problem by giving similar functionalities but in a more portable version.

---

## Supported Features

### 1. Project Files & Startup Launcher
- **Startup Project Selector**: Opens with a searchable project launcher listing recently opened `.Loxone` files, workspace projects, and local pre-upload backups (or pass a `.Loxone` path via CLI). Switch projects anytime by clicking the project title in the top bar or via the command palette (`Cmd+K` / `Ctrl+K`).
- **Native `.Loxone` & `.LoxCC` Support**: Reads and writes UTF-8 BOM `.Loxone` XML (`Document Type="LoxLIVE"`) as well as compressed `.LoxCC` binary/ZIP containers used by the Miniserver.
- **Lossless XML Serialization**: Edits are applied in-place to the underlying XML DOM so untouched blocks, attributes, and custom child nodes round-trip cleanly without data loss.
- **Unsaved Changes Guard**: Tracks dirty project state (`*`) and prompts to **Save & Continue**, **Discard Changes**, or **Cancel** before switching projects or loading a different file.

### 2. Interactive Schematic Canvas & Authoring
- **Loxone Knowledge Base Block Catalog (83+ Schemas)**: Built-in catalog of function blocks and periphery items across Lighting, Shading, Climate, Audio, Security, Energy, Analog/Digital Logic, Counter/Timing, and System categories.
- **Full Schematic Editing**:
  - Add, move, resize (`Memo` boxes), and delete function blocks, I/O reference bars (`InputRef`, `OutputRef`, `OutputRefLM`), and memory flags (`Memory`).
  - Drag-to-connect pins with obstacle-avoiding orthogonal or Bezier wire routing.
  - Multi-selection via marquee drag or `Shift`/`Cmd`-click with group movement and deletion.
  - Copy (`Cmd+C`), Cut (`Cmd+X`), Paste (`Cmd+V`), and Duplicate (`Cmd+D`) with internal wire preservation and automatic UUID generation.
  - Full reversible **Undo / Redo** (`Cmd+Z` / `Cmd+Shift+Z`) across block moves, wiring, parameter edits, and structural changes.
- **Cross-Page References**: Right-click any output pin and choose **Send to Page…** to automatically create a linked `OutputRef` + `InputRef` pair on a target page, or click cross-page badges (`-> Page`) to jump directly to the counterpart block.
- **Always-Visible Unit & Pin Badges**: Compact reference bars display their Loxone unit/pin type (`T5`, `Br`, `P`, `ϑc`, `H`, `M`, `VQ`, `Q14`, `->`) directly on the canvas.
- **Inline Rename**: Double-click any block on the canvas or any page, room, category, or periphery item in the left tree to rename it in place.

### 3. Project Tree & Navigation
- **5-Section Project Tree**:
  - **Periphery**: Miniserver, Extensions, Tree/Air/Link buses, Modbus/1-Wire/DALI devices, and physical I/O channels with custom vector icons and technical subtitles.
  - **Virtual I/O & Memory**: Virtual Inputs (`VirtualIn`), Virtual Outputs (`VirtualOut` / `VirtualOutCmd`), and Memory Flags (`Merker`) with quick `+` creation buttons.
  - **Pages**: All schematic pages with block counts and `+` / delete / rename controls.
  - **Rooms (`Places`) & Categories**: Filter or inspect blocks by room and category, or create/edit rooms and categories.
- **Command Palette (`Cmd+K` / `Ctrl+K`)**: Instant fuzzy search across all pages, function blocks, periphery devices, rooms, categories, and application commands.

### 4. Property Inspector & Specialized Block Editors
- **Knowledge Base Integration**: Displays localized block type names, pin documentation, and direct links (`📖 Loxone Docs`) to official Loxone documentation.
- **Typed Parameter & Attribute Editors**: Edit block titles, room/category assignments (`<IoData>`), visualization settings (`Visu`, `VisuPwd`), operating modes, and default pin parameters (`<Co Def="...">`).
- **Specialized Configuration Editors**:
  - **`LightControllerV2`**: Interactive lighting scene (`Mood`) editor and actuator channel assignments.
  - **`IRC` / `IRCV2` (Intelligent Room Controller)**: Comfort heating/cooling setpoints, eco offsets, and operating parameters.
  - **`Radio` (Radio Buttons)**: Named output mapping table (`O1`–`O16`).
  - **`Status`**: Condition rules, comparison operators, and status output texts/values.
  - **Child Config Sections**: Dedicated editors for `<LoggerMailer>`, `<HCL>` (Human Centric Lighting), `<PSD>` (Presence/Daylight), and `<Display>`.
- **Inline Linked Target Editing**: Selecting an `InputRef`, `OutputRef`, or `Memory` bar lets you edit the underlying periphery or virtual I/O target directly from the inspector.

### 5. Live View (`⚡ Live`) & On-Canvas Simulation
- **Real-Time Miniserver Polling & Offline Simulation**: Toggle **⚡ Live** in the top bar to poll live I/O states from a connected Miniserver (`/dev/sps/io/<uuid>/all`) or test logic interactively offline with automatic signal propagation across wires and references.
- **Energized Wire Visualization**: Active digital (`> 0`) and analog wires glow in Loxone green (`#59B828`) with inline live value badges (`On`, `Off`, `21.5`, etc.).
- **Interactive On-Canvas Controls**:
  - **Toggle Switch**: Pill switch (`On` / `Off`) inside digital inputs and memory flags.
  - **Pushbutton (`Pulse`)**: Momentary trigger button inside impulse/pushbutton inputs.
  - **T5 Touch 5-Zone Pad**: Interactive 5-button Loxone Touch layout (`Center`, `TL`, `TR`, `BL`, `BR`) encoding bitmask values (`1`, `2`, `4`, `8`, `16`).
  - **Analog Value Popover**: Clickable numeric badge to enter or adjust analog values (temperatures, brightness, setpoints) directly on the canvas.

### 6. Miniserver Download & Safe Hardware Deployment
- **Download from Miniserver**: Connect via HTTP to inspect `/dev/fslist/prog/`, download the latest configuration or choose any historical `.LoxCC` backup snapshot on the Miniserver, and automatically decompress it into the editor.
- **Gen1 & Gen2 Authentication**: Supports both HTTP Basic authentication and Loxone token/key-exchange authentication (`/jdev/sys/getkey2`, HMAC-SHA1/SHA256).
- **Safe Upload to Miniserver**:
  - Automatically downloads and saves a timestamped local backup (`~/.config/loxone-portable/backups/`) of the Miniserver's current configuration before uploading.
  - Compresses the edited project into a valid `.LoxCC` archive, uploads it to `/prog/`, and optionally triggers `/dev/sps/restart` to activate the new program.

### 7. Project Diagnostics & Linter (`✓ Verify`)
- **One-Click Project Verification**: Click **✓ Verify** in the top toolbar to scan the entire project for:
  - Dangling or broken `Ref` / `RefCaller` UUID links.
  - Blocks missing Room (`Pr`) or Category (`Cr`) assignments where required by the block schema.
  - Isolated function blocks with zero connected inputs or outputs.
- **Click-to-Jump Navigation**: Click any diagnostic warning or error to jump directly to the affected page and select the block.

---

## Keyboard Shortcuts

| Shortcut (macOS / Windows & Linux) | Action |
| :--- | :--- |
| `Cmd+K` / `Ctrl+K` | Open Command Palette & global search |
| `Cmd+O` / `Ctrl+O` | Open `.Loxone` file from disk |
| `Cmd+S` / `Ctrl+S` | Save current `.Loxone` project |
| `Cmd+I` / `Ctrl+I` | Insert Function Block / Reference modal |
| `Cmd+Z` / `Ctrl+Z` | Undo last edit |
| `Cmd+Shift+Z` / `Ctrl+Y` | Redo |
| `Cmd+C` / `Ctrl+C` | Copy selected blocks |
| `Cmd+X` / `Ctrl+X` | Cut selected blocks |
| `Cmd+V` / `Ctrl+V` | Paste blocks (preserves internal wiring) |
| `Cmd+D` / `Ctrl+D` | Duplicate selected blocks |
| `Delete` / `Backspace` | Delete selected blocks or wire |
| `Double-Click` (block or tree item) | Inline rename |
| `Scroll` / `Pinch` | Pan / Zoom canvas |

---

## Installation & Building

### Prebuilt Binaries
Prebuilt binaries for **macOS** (Apple Silicon & Intel `.app` bundles), **Windows** (`x86_64`), and **Linux** (`x86_64`) are automatically built via GitHub Actions on every release (`v*` tags). Check the **Releases** page to download the latest archive for your OS.

### Build from Source
Ensure you have the [Rust toolchain](https://rustup.rs/) installed, then run:

```bash
# Run in release mode (opens the Project Launcher)
cargo run --release -p loxone-gui

# Or open a specific .Loxone project directly
cargo run --release -p loxone-gui -- path/to/Project.Loxone

# Run the test suite
cargo test --workspace
```
