# Loxone Portable — Agent Context & Progress (`agent.md`)

Cross-platform desktop viewer, schematic editor, and live Miniserver inspector for `.Loxone` project files (`Document Type="LoxLIVE"` XML and compressed `.LoxCC` archives), built in Rust (`eframe` / `egui` 0.31).

## Workspace Structure

- **`crates/loxone-core`** — Pure data & protocol library (zero GUI dependencies):
  - `src/model.rs` — Project DOM (`LoxoneProject`, `Page`, `PageBlock`, `TreeNode`, `ConnectorPin`, `Place`, `Category`, `RawNode`).
  - `src/parser.rs` & `src/loxcc.rs` — UTF-8 BOM `.Loxone` XML parser and `.LoxCC` LZ4-style decompression/ZIP extraction.
  - `src/xref.rs` — Cross-reference index (`XrefIndex`) resolving `Ref` targets, cross-page links, rooms, and categories.
  - `src/block_schema.rs` & `src/factory.rs` — Loxone Knowledge Base catalog (83+ function block & periphery schemas) and UUID/element factory.
  - `src/writer.rs` — Structural `ProjectEdit`, reversible `apply_edit_with_inverse` (undo/redo), and lossless XML round-trip serializer.
  - `src/miniserver.rs` — Miniserver HTTP client (`/dev/fslist/prog/`, `/dev/sps/io/<uuid>/all`), `LiveValueStore`, signal propagation, and `RefBarLiveControlKind`.
- **`crates/loxone-gui`** — Desktop GUI (`eframe` / `egui` 0.31):
  - `src/app.rs` — Main application state, undo/redo history, background Miniserver worker, and Live View polling/command dispatch.
  - `src/canvas.rs` — Interactive schematic canvas: pan/zoom, obstacle-avoiding orthogonal/Bezier wires, multi-selection (marquee + Shift/Cmd-click), group drag/delete, pin wiring, always-visible small-block unit capsules, and interactive on-canvas Live View controls (`ToggleSwitch`, `Pushbutton`, `NumericValue`, `Touch5Pad`).
  - `src/tree_panel.rs` & `src/icons.rs` — 5-section project navigation tree with custom vector device/bus/sensor/block icons and technical subtitles.
  - `src/inspector.rs` — Right-hand property inspector with KB docs, typed parameter editors, inline linked target properties, and live I/O values.
  - `src/top_bar.rs`, `src/insert_modal.rs`, `src/command_palette.rs`, `src/miniserver_modal.rs` — Declarative 4-group toolbar, block/reference insert modal, `Cmd+K` search palette, and Miniserver connection modal.

## Development & Verification

- **Primary real-world test fixture:** `Klimczaki.Loxone` (28 pages, 829 blocks, 35 rooms, 13 categories).
- **Run all tests (must pass with 0 warnings):**
  ```bash
  export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
  cargo test --workspace
  ```

## Completed Progress

1. **Core `.Loxone` & `.LoxCC` Engine + Miniserver Download** — Parses full `.Loxone` XML and compressed Miniserver `/prog/` backups; downloads and saves projects directly from a Loxone Miniserver.
2. **Lossless Editing & Loxone KB Property Inspector** — Full attribute, pin parameter (`<Co Def="...">`), and child config (`<IoData>`, `<LoggerMailer>`, `<HCL>`, `<PSD>`, `<Display>`) editing with lossless XML serialization.
3. **Interactive Schematic Authoring & Multi-Selection** — Add/delete/rename pages, rooms, categories, periphery nodes, function blocks, and wires; marquee box selection, multi-block group drag & batch delete, and `Cmd+Z` / `Cmd+Shift+Z` undo/redo.
4. **Visual Polish (Vector Icons & Top Toolbar)** — Custom vector icons across the periphery tree and canvas headers, 1.35x readable base canvas scale, and a structured 4-group top toolbar (`File & Miniserver`, `History & Insert`, `Canvas View`, `Breadcrumb + Live Status + Search`).
5. **Live View & Interactive Small-Block Controls** — Live View mode (`⚡ Live`) with real-time Miniserver `/dev/sps/io/` polling and offline simulation, energized Loxone-green (`#59B828`) wires with inline value labels (`Wył.`, `Wł.`, `22.9°`, `0Lx`), always-visible unit/pin capsules (`T5`, `Br`, `P`, `ϑc`, `H`, `M`, `VQ`, `Q14`, `->`) in both Live OFF and Live ON states, and on-canvas interactive controls inside small blocks (toggle switches, pushbuttons, float/number popover, and T5 touch pad simulator) with per-channel isolation.
