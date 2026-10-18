# Contributing

Thanks for helping with Loxone Portable. The project is in alpha — keep a backup of any `.Loxone` file before saving or uploading it to a live Miniserver.

## Setup

Install the [Rust toolchain](https://rustup.rs/), then:

```bash
cargo run --release -p loxone-gui
cargo test --workspace
```

## Layout

- `crates/loxone-core` — project files, edits, and Miniserver protocol (no GUI)
- `crates/loxone-gui` — desktop app (`eframe` / `egui`) and locale files in `locales/`

## Pull requests

1. Open an issue for larger changes so the approach can be agreed first.
2. Keep the change focused.
3. Run `cargo fmt` and `cargo test --workspace` before opening the PR.

Contributions are licensed under the project license (MIT).
