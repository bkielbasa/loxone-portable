# Miniserver Connection, Status Indicator, Extensible Top Bar & Project Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Loxone Miniserver connection verification, `.LoxCC` / `.zip` backup listing and project download, `.Loxone` XML saving, and an extensible top bar with a live connection status icon.

**Architecture:** `loxone-core` gains `loxcc.rs` (LZ4-style `0xAABBCCEE` `.LoxCC` unpacker + ZIP archive extractor), `miniserver.rs` (`ConnectionStatus`, `/dev/fslist/prog` snapshot parser, `ConnectionProfile` persistence, and `MiniserverClient` over HTTP/HTTPS + FTP fallback), and `LoxLIVE` metadata extraction in `model.rs` / `parser.rs`. `loxone-gui` extracts an extensible `top_bar.rs` with declarative action groups and a color-coded Miniserver connection status icon, adds `miniserver_modal.rs` with background worker thread integration, and wires `ConnectMiniserver` and `SaveFile` into `command_palette.rs` and `app.rs`.

**Tech Stack:** Rust 2021, `eframe` / `egui` 0.31, `zip` 2.2, `ureq` 2.12, `suppaftp` 6.0, `base64` 0.22, `serde` / `serde_json` 1.0.

**Spec:** `docs/superpowers/specs/2026-10-15-miniserver-download-design.md`

## Global Constraints

- **Toolchain PATH:** Prefix cargo commands with `export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"`.
- **Zero Warnings:** `cargo test --workspace` must pass with zero compiler warnings.
- **Minimalist Aesthetic:** Preserve `#111317` canvas, `#161920` panels, `#1B1F2A` cards, `#242936` `1px` borders, `#5E81FF` accent blue, and semantic status colors (`#8B95A7` Disconnected, `#F59E0B` Connecting, `#3ECF8E` Connected, `#E5534B` Error).

---

### Task 1: `LoxLIVE` Metadata Extraction & `.LoxCC` Archive Decompression (`loxone-core`)

**Files:**
- Modify: `crates/loxone-core/Cargo.toml`
- Modify: `crates/loxone-core/src/lib.rs`
- Modify: `crates/loxone-core/src/model.rs`
- Modify: `crates/loxone-core/src/parser.rs`
- Create: `crates/loxone-core/src/loxcc.rs`

**Interfaces:**
- Produces:
  - `ProjectMetadata::{miniserver_ip, miniserver_serial, miniserver_hostname}: Option<String>`
  - `LoxoneProject::raw_xml: Option<String>`
  - `loxone_core::loxcc::{LoxCcError, LOXCC_MAGIC, decompress_loxcc, extract_loxone_xml, compress_loxcc_literals}`

- [ ] **Step 1: Write failing tests for `LoxLIVE` metadata, `raw_xml`, and `loxcc` decompression**
- [ ] **Step 2: Run `cargo test -p loxone-core` to verify RED failure**
- [ ] **Step 3: Implement `loxcc.rs` and `LoxLIVE` extraction in `model.rs` / `parser.rs`**
- [ ] **Step 4: Run `cargo test -p loxone-core` to verify GREEN pass**
- [ ] **Step 5: Commit Task 1**

---

### Task 2: Miniserver Connection Status, Snapshot Listing, Profile & Client (`crates/loxone-core/src/miniserver.rs`)

**Files:**
- Modify: `crates/loxone-core/Cargo.toml`
- Modify: `crates/loxone-core/src/lib.rs`
- Create: `crates/loxone-core/src/miniserver.rs`

**Interfaces:**
- Consumes: `loxone_core::loxcc::extract_loxone_xml`
- Produces:
  - `loxone_core::miniserver::{ConnectionStatus, BackupSnapshot, ConnectionProfile, MiniserverCredentials, MiniserverError, MiniserverClient, parse_prog_listing}`

- [ ] **Step 1: Write failing tests for `parse_prog_listing`, `ConnectionProfile` password stripping, and mock HTTP server `check_connection` / `list_snapshots` / `download_snapshot_xml`**
- [ ] **Step 2: Run `cargo test -p loxone-core` to verify RED failure**
- [ ] **Step 3: Implement `crates/loxone-core/src/miniserver.rs`**
- [ ] **Step 4: Run `cargo test -p loxone-core` to verify GREEN pass**
- [ ] **Step 5: Commit Task 2**

---

### Task 3: Extensible Top Bar with Connection Status Icon, Miniserver Modal & App Integration (`loxone-gui`)

**Files:**
- Modify: `crates/loxone-gui/src/theme.rs`
- Create: `crates/loxone-gui/src/top_bar.rs`
- Create: `crates/loxone-gui/src/miniserver_modal.rs`
- Modify: `crates/loxone-gui/src/command_palette.rs`
- Modify: `crates/loxone-gui/src/app.rs`
- Modify: `crates/loxone-gui/src/main.rs`

**Interfaces:**
- Consumes: `loxone_core::miniserver::*`, `loxone_core::parser::parse_loxone_xml`
- Produces:
  - `crate::top_bar::{TopBarAction, ToolbarGroup, ToolbarItem, StatusBadgeVisuals, status_badge_visuals, build_miniserver_toolbar_items, render_top_bar}`
  - `crate::miniserver_modal::{MiniserverModalState, MiniserverModalAction, render_miniserver_modal}`
  - Updated `PaletteAction::{ConnectMiniserver, SaveFile}` and `LoxonePortableApp` background worker channel polling

- [ ] **Step 1: Write failing tests in `top_bar.rs`, `miniserver_modal.rs`, `command_palette.rs`, and `app.rs`**
- [ ] **Step 2: Run `cargo test -p loxone-gui` to verify RED failure**
- [ ] **Step 3: Implement `top_bar.rs`, `miniserver_modal.rs`, and integrate into `theme.rs`, `command_palette.rs`, `app.rs`, and `main.rs`**
- [ ] **Step 4: Run `cargo test --workspace` to verify GREEN pass with zero warnings**
- [ ] **Step 5: Commit Task 3**
