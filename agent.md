# Loxone Portable — Agent Context & Progress (`agent.md`)

Cross-platform desktop viewer, schematic editor, and live Miniserver inspector for `.Loxone` project files (`Document Type="LoxLIVE"` XML and compressed `.LoxCC` archives), built in Rust (`eframe` / `egui` 0.31).

## Workspace Structure

- **`crates/loxone-core`** — Pure data & protocol library (zero GUI dependencies):
  - `src/model.rs` — Project DOM (`LoxoneProject`, `Page`, `PageBlock`, `TreeNode`, `ConnectorPin`, `Place`, `Category`, `RawNode`).
  - `src/parser.rs` & `src/loxcc.rs` — UTF-8 BOM `.Loxone` XML parser and `.LoxCC` LZ4-style decompression + compression/ZIP packaging.
  - `src/xref.rs` — Cross-reference index (`XrefIndex`) resolving `Ref` targets, cross-page links, rooms, and categories.
  - `src/block_schema.rs` & `src/factory.rs` — Loxone Knowledge Base catalog (83+ function block & periphery schemas) and UUID/element factory.
  - `src/writer.rs` — Structural `ProjectEdit`, reversible `apply_edit_with_inverse` (undo/redo), resize/paste/cross-page link helpers, and lossless XML round-trip serializer.
  - `src/linter.rs` — Project diagnostics & linter (`lint_project`) detecting dangling references, missing rooms/categories, and isolated blocks.
  - `src/miniserver.rs` — Miniserver HTTP client (`/dev/fslist/prog/`, `/dev/sps/io/<uuid>/all`), Gen1/Gen2 token authentication (`getkey2`), safe project upload with automatic local backup (`upload_project`), `LiveValueStore`, signal propagation, and `RefBarLiveControlKind`.
- **`crates/loxone-gui`** — Desktop GUI (`eframe` / `egui` 0.31):
  - `src/app.rs` — Main application state, undo/redo history, clipboard (`Cmd+C`/`Cmd+X`/`Cmd+V`/`Cmd+D`), unsaved changes guard, background Miniserver worker, and Live View polling/command dispatch.
  - `src/project_launcher.rs` — Startup project selector listing discovered `.Loxone` files from recent history, workspace, and local backups.
  - `src/canvas.rs` — Interactive schematic canvas: pan/zoom, obstacle-avoiding orthogonal/Bezier wires, multi-selection (marquee + Shift/Cmd-click), group drag/delete, inline double-click rename, resizable `Memo` boxes, guided cross-page `OutputRef`/`InputRef` creation, always-visible small-block unit capsules, and interactive on-canvas Live View controls (`ToggleSwitch`, `Pushbutton`, `NumericValue`, `Touch5Pad`).
  - `src/tree_panel.rs` & `src/icons.rs` — 5-section project navigation tree with custom vector device/bus/sensor/block icons, inline double-click rename, and technical subtitles.
  - `src/inspector.rs` — Right-hand property inspector with KB docs, typed parameter editors, specialized editors (`LightControllerV2` scenes, `IRC`/`IRCV2` heating/cooling setpoints, `Radio` table, `Status` rules), inline linked target properties, and live I/O values.
  - `src/top_bar.rs`, `src/insert_modal.rs`, `src/command_palette.rs`, `src/miniserver_modal.rs` — Declarative 4-group toolbar (with `✓ Verify`), block/reference insert modal, `Cmd+K` search palette, and Miniserver download/upload modal.

## Development & Verification

- **Primary real-world test fixture:** `Klimczaki.Loxone` (28 pages, 829 blocks, 35 rooms, 13 categories).
- **Run all tests (must pass with 0 warnings):**
  ```bash
  export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
  cargo test --workspace
  ```

## Completed Progress

1. **Core `.Loxone` & `.LoxCC` Engine + Miniserver Download & Safe Upload** — Parses and round-trip serializes `.Loxone` XML and compressed `.LoxCC` archives; supports Gen1/Gen2 token authentication, automatic pre-upload local backups, and uploading `.LoxCC` archives with optional `/dev/sps/restart`.
2. **Startup Project Launcher (`project_launcher.rs`)** — Opens to a searchable project list on startup (recent files, workspace `.Loxone` projects, and local backups), and allows switching projects at any time by clicking the project title in the top bar or via `Cmd+K`.
3. **Lossless Editing, Specialized Block Editors & Project Diagnostics (`✓ Verify`)** — Full attribute, pin parameter (`<Co Def="...">`), child config (`<IoData>`, `<LoggerMailer>`, `<HCL>`, `<PSD>`, `<Display>`), and specialized block editors (`LightControllerV2`, `IRC`/`IRCV2`, `Radio`, `Status`), plus a project linter modal (`✓ Verify`) with click-to-jump navigation.
4. **Daily Schematic Editing Ergonomics** — Copy/Cut/Paste/Duplicate (`Cmd+C`/`Cmd+X`/`Cmd+V`/`Cmd+D`) with internal wire preservation, unsaved-changes confirmation modal, double-click inline rename on canvas & tree, resizable `Memo` boxes, and right-click `Send to Page…` cross-page `OutputRef`/`InputRef` creation.
5. **Live View & Interactive Small-Block Controls** — Live View mode (`⚡ Live`) with real-time Miniserver `/dev/sps/io/` polling and offline simulation, energized Loxone-green (`#59B828`) wires with inline value labels, always-visible unit/pin capsules (`T5`, `Br`, `P`, `ϑc`, `H`, `M`, `VQ`, `Q14`, `->`), and on-canvas interactive controls inside small blocks.
