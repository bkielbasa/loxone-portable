# Loxone Portable — Agent Context & Progress (`agent.md`)

Cross-platform desktop viewer, schematic editor, and live Miniserver inspector for `.Loxone` project files (`Document Type="LoxLIVE"` XML and compressed `.LoxCC` archives), built in Rust (`eframe` / `egui` 0.31).

## Workspace Structure

- **`crates/loxone-core`** — Pure data & protocol library (zero GUI dependencies):
  - `src/model.rs` — Project DOM (`LoxoneProject`, `Page`, `PageBlock`, `TreeNode`, `ConnectorPin`, `Place`, `Category`, `RawNode`).
  - `src/parser.rs` & `src/loxcc.rs` — UTF-8 BOM `.Loxone` XML parser and `.LoxCC` LZ4-style decompression + compression/ZIP packaging.
  - `src/xref.rs` — Cross-reference index (`XrefIndex`) resolving `Ref` targets, cross-page links, rooms, and categories.
  - `src/block_schema.rs` & `src/factory.rs` — Loxone Knowledge Base catalog (220 function block & periphery schemas) and UUID/element factory.
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
- **`crates/loxone-format-lab`** — Corpus-driven reverse-engineering CLI (depends only on `loxone-core`):
  - `src/corpus.rs` & `src/stats.rs` — Discovery/loading of `.Loxone`, `.LoxCC`, and Miniserver `.zip` files, plus aggregation of types, attributes, pins, contexts, and child elements.
  - `src/diff.rs` — UUID-based structural diff between two saves (attributes, pin defaults, wires, child configs, nested `IoData` fields).
  - `src/report.rs` & `src/gen.rs` — Schema coverage reports (`scan`, `check`) and conservative starter `schema!` generation for unknown types.
  - `src/main.rs` — `scan`, `dump`, `diff`, `check`, and `gen` subcommands.
  - `fixtures/` — Anonymised synthetic projects used by the lab's own tests.

## Research Workflow

Use the format lab for any format/behavior work that cannot be answered from the
existing code. The full workflow lives in
[`docs/reverse-engineering.md`](docs/reverse-engineering.md).

- **Corpus location:** `corpus/` at the repo root is git-ignored. Put real
  `.Loxone`/`.LoxCC` projects, Miniserver snapshots, reports, and local
  decompiled metadata there.
- **Never commit** customer project data, Miniserver credentials, or decompiled
  proprietary Loxone Config code. Only anonymised synthetic fixtures belong in
  `crates/loxone-format-lab/fixtures/`.
- **Typical loop:** observe with `scan`, isolate one edit with `diff`, inspect
  `dump`, draft a schema with `gen`, add meaning from the official KB or
  decompiled metadata, then validate by round-tripping through `loxone-core`,
  opening the result in Loxone Config, and uploading to a test Miniserver.
- **Commands** (run from the repo root):
  ```bash
  cargo run -p loxone-format-lab -- scan corpus/projects --out corpus/reports/scan.md
  cargo run -p loxone-format-lab -- check corpus/projects
  cargo run -p loxone-format-lab -- diff before.Loxone after.Loxone
  cargo run -p loxone-format-lab -- dump project.Loxone --uuid <uuid>
  cargo run -p loxone-format-lab -- gen corpus/projects --out corpus/reports/new-types.rs
  ```

## Development & Verification

- **Primary real-world test fixture:** `Klimczaki.Loxone` (28 pages, 829 blocks, 35 rooms, 13 categories). It is intentionally not committed; place it at the repo root to run the core/GUI tests that reference it.
- **Run all tests (must pass with 0 warnings):**
  ```bash
  export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
  cargo test --workspace
  ```
- **Format-lab tests do not need the real fixture:**
  ```bash
  cargo test -p loxone-format-lab
  ```

## Completed Progress

1. **Core `.Loxone` & `.LoxCC` Engine + Miniserver Download & Safe Upload** — Parses and round-trip serializes `.Loxone` XML and compressed `.LoxCC` archives; supports Gen1/Gen2 token authentication, automatic pre-upload local backups, and uploading `.LoxCC` archives with optional `/dev/sps/restart`.
2. **Startup Project Launcher (`project_launcher.rs`)** — Opens to a searchable project list on startup (recent files, workspace `.Loxone` projects, and local backups), and allows switching projects at any time by clicking the project title in the top bar or via `Cmd+K`.
3. **Lossless Editing, Specialized Block Editors & Project Diagnostics (`✓ Verify`)** — Full attribute, pin parameter (`<Co Def="...">`), child config (`<IoData>`, `<LoggerMailer>`, `<HCL>`, `<PSD>`, `<Display>`), and specialized block editors (`LightControllerV2`, `IRC`/`IRCV2`, `Radio`, `Status`), plus a project linter modal (`✓ Verify`) with click-to-jump navigation.
4. **Daily Schematic Editing Ergonomics** — Copy/Cut/Paste/Duplicate (`Cmd+C`/`Cmd+X`/`Cmd+V`/`Cmd+D`) with internal wire preservation, unsaved-changes confirmation modal, double-click inline rename on canvas & tree, resizable `Memo` boxes, and right-click `Send to Page…` cross-page `OutputRef`/`InputRef` creation.
5. **Live View & Interactive Small-Block Controls** — Live View mode (`⚡ Live`) with real-time Miniserver `/dev/sps/io/` polling and offline simulation, energized Loxone-green (`#59B828`) wires with inline value labels, always-visible unit/pin capsules (`T5`, `Br`, `P`, `ϑc`, `H`, `M`, `VQ`, `Q14`, `->`), and on-canvas interactive controls inside small blocks.
6. **Format Research Lab (`loxone-format-lab`)** — Standalone corpus-driven CLI for schema/format reverse engineering: corpus statistics, structural before/after diffs, raw node dumps, schema coverage checks, and starter schema generation. Public core hooks (`parse_raw_dom`, `serialize_raw_node(s)`, `all_block_schemas()`) support the lab without affecting the GUI. The catalog now has 220 schemas; the public two-project corpus observes 132 types with zero missing block/periphery types, leaving 165 attribute-detail gaps for the next corpus pass.
