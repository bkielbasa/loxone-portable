# Ticket: Implement block type `BenchSensor`

## Status
- [ ] Schema (attributes, pins, child configs)
- [ ] Rust structs / model
- [ ] Parser / writer round-trip
- [ ] Periphery provisioning
- [ ] GUI (icon, canvas, config modal)
- [ ] Tests
- [ ] Corpus evidence

## Context
Candidate block type discovered in the Loxone Config binary string table
(`LoxoneConfig/LoxoneConfig.exe`). A placeholder `tree_schema!` entry was added
to `crates/loxone-core/src/block_schema.rs` (TREE_SCHEMAS_3) but no attributes,
pins, or child configs are defined yet.

## Needed
A real `.Loxone` project that uses this block type, so the corpus pipeline
(`scan`, `check`, `gen --include-known`) can discover its attributes, pins,
and child elements. Please attach such a project (anonymized) to this ticket.

## References
- crates/loxone-core/src/block_schema.rs (TREE_SCHEMAS_3)
- crates/loxone-format-lab (scan/check/gen)
- docs/reverse-engineering.md
