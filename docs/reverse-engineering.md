# Reverse-engineering workflow (`loxone-format-lab`)

`loxone-format-lab` is a corpus-driven research CLI. It does not try to guess
what a block does from its name: it measures what real Loxone projects contain,
diffs two saves to reveal the exact effect of an edit, and reports where the
baked-in knowledge base is incomplete.

The GUI and the core library stay unchanged; the lab is a separate binary that
only depends on `loxone-core`.

## 1. Collect a corpus

Create a local, **git-ignored** corpus:

```text
corpus/
  projects/        # .Loxone and .LoxCC files from several Config versions
  miniserver/      # downloaded sps0.LoxCC / .zip snapshots
  loxone-config/   # local decompiled metadata (never commit)
  reports/         # generated Markdown/JSON reports
```

Copy projects out of the Windows VM (shared folder, USB, or `scp`). Useful
sources:

- projects saved by Loxone Config, ideally from at least two Config versions;
- projects with different hardware (Miniserver Gen 1/Gen 2, Tree, Air, Modbus,
  DMX, 1-Wire, Audio Server);
- Miniserver snapshots downloaded through the app (`.LoxCC`) or via
  `/dev/fsget/prog/...`;
- the real Loxone Config installation (`LoxoneConfig.exe` and its `.dll` files).

Keep customer data out of git. `/corpus/` is already in `.gitignore`.

## 2. Build

```bash
source "$HOME/.cargo/env"          # Rust was installed with rustup in this workspace
cargo build -p loxone-format-lab
```

The binary is `target/debug/loxone-format-lab`.

## 3. Survey the corpus

```bash
# Human-readable Markdown report (schema coverage, attributes, pins, contexts)
cargo run -p loxone-format-lab -- scan corpus/projects --out corpus/reports/scan.md

# Machine-readable JSON for further processing
cargo run -p loxone-format-lab -- scan corpus/projects --json > corpus/reports/scan.json

# Only show schema gaps
cargo run -p loxone-format-lab -- check corpus/projects

# CI-style failure if anything is missing a schema
cargo run -p loxone-format-lab -- check corpus/projects --fail
```

For every observed XML `Type`, the scan records:

- how often it occurs and in which section (`Program`, `LoxLIVE`, ...);
- its parent types;
- every attribute, fill rate, distinct count, examples, and a coarse inferred
  kind (`bool`, `int`, `float`, `uuid`, `color`, `enum`, `text`);
- every `Co` pin key, default values, incoming links, hidden/detached flags, and
  whether the pin acts as a wire source (i.e. is an output);
- child elements and their attributes (`IoData.Pr`, `HCL`, `LoggerMailer`, ...).

`check` compares those observations with `crates/loxone-core/src/block_schema.rs`
and separates:

- **actionable gaps** — unknown block/periphery types, their attributes, pins,
  and child elements;
- **non-block XML types** — metadata, captions, icon tables, and document
  sections that do not need block schemas;
- **universal attributes** — common fields such as `U`, `Title`, `Px`, `Ref`,
  which are expected on most controls and are not counted as schema gaps.

A type is only treated as a schema candidate when it carries connector pins or
sits directly on a page. Use `gen --include-non-block` when you explicitly want
metadata types too. `check --fail` exits non-zero only for actionable gaps.

## 4. Reverse-engineer one edit at a time

The fastest way to learn a field is to make exactly one change in Loxone Config
and diff the before/after files:

```bash
# Save a copy before editing, edit in Loxone Config, save a second copy.
cargo run -p loxone-format-lab -- diff \
  corpus/projects/before.Loxone \
  corpus/projects/after.Loxone \
  --out corpus/reports/light-edit.md

# Focus on one block type
cargo run -p loxone-format-lab -- diff before.Loxone after.Loxone \
  --only-type LightController2 --json
```

The diff matches nodes by UUID and reports:

- added/removed blocks and tree nodes;
- changed attributes (`Title`, `Nio`, ...);
- pin default changes (`Co[I1].Def`);
- added/removed wires (`Co[I1].In`, including `FLG=1/2`);
- added/removed child elements (`IoData`, `HCL`, `PSD`, `LoggerMailer`, ...);
- nested child attributes such as `IoData.Pr` / `IoData.Cr`.

Save levels (`SL`), `DateS`, and `CDate` are ignored by default because they
change on every save. Add more noise with `--ignore-attr`.

Inspect a node directly when the diff is not enough:

```bash
cargo run -p loxone-format-lab -- dump after.Loxone --uuid <uuid>
cargo run -p loxone-format-lab -- dump after.Loxone --type LightController2 --limit 5
cargo run -p loxone-format-lab -- dump after.Loxone --path Program/C[Page] --tag IoData
```

## 5. Generate a starter schema

```bash
# Generate code for every type that has no schema yet
cargo run -p loxone-format-lab -- gen corpus/projects --out corpus/reports/new-types.rs

# Or for one type, including an existing schema
cargo run -p loxone-format-lab -- gen corpus/projects --type LightController2 --include-known

# Regenerate all known block/periphery schemas for comparison
cargo run -p loxone-format-lab -- gen corpus/projects --include-known --out corpus/reports/known-types.rs
```

The generated `schema!` entries include observed attributes, inferred kinds,
stable defaults, pin parameters, input/output docs, and known child configs.
They are deliberately conservative:

- labels fall back to XML names;
- enum labels are the raw values;
- descriptions say "auto-generated";
- unknown child elements are emitted as TODO comments.

Review the snippet against the official Loxone KB and the decompiled Config
metadata before pasting it into `block_schema.rs`.

## 6. Decompiling Loxone Config (metadata only)

Loxone Config is a Windows .NET application. On a Windows machine (or a Windows
VM) install [ILSpy](https://github.com/icsharpcode/ILSpy) or `ilspycmd`:

```powershell
dotnet tool install -g ilspycmd
ilspycmd "C:\Program Files\Loxone\Loxone Config\LoxoneConfig.exe" -o C:\loxone-decompiled
```

Search the decompiled sources for:

- XML attribute names discovered by `scan` (`TE`, `Nio`, `WF`, ...);
- block type names (`LightController2`, `IRC`, ...);
- default values and validation ranges;
- enum label tables and bitmask flags;
- Miniserver endpoints used for Tree/Air pairing and device discovery.

Use this to attach **meaning** to the XML evidence: labels, units, ranges,
defaults, enum labels, and validation rules. Do not commit decompiled source or
customer projects. Record only the derived field mappings.

## 7. Miniserver evidence

For device provisioning and live behaviour, pair the corpus with Miniserver
observations:

- download the current `sps0.LoxCC` before and after a Config operation;
- diff the two snapshots (`diff` also accepts two `.LoxCC` files);
- capture the HTTP calls Loxone Config makes during device search/pairing
  (browser dev tools, `mitmproxy`, or the Miniserver's own logs);
- compare a project where a device is paired with one where it is not.

This is the evidence needed to replace `generate_mock_serial()` and the
hand-built Tree/Air nodes in `periphery_provisioning.rs`.

## 8. Validation loop

For every new field or writer operation:

1. Apply it through `loxone-core` and serialize with
   `serialize_loxone_xml`.
2. Re-parse and confirm the DOM round-trips.
3. Open the result in the real Loxone Config and confirm it is accepted.
4. Upload to a **test** Miniserver and check that the device/block works.
5. Diff the Config-saved result against your output; aim for a minimal,
   semantically equivalent delta.

## Public corpus baseline

The repository ships no real projects. Two public sample `.Loxone` files were
used as the initial corpus (`applamp-mighlight-example.Loxone` and
`wallbox-reporting-beispiel.Loxone`) and produced this baseline:

- 2 files, 11 pages, 127 page blocks, 1419 connector pins, 132 observed types;
- every observed block/periphery type now has a schema (220 schemas total);
- 16 remaining unknown types are metadata/captions/sections, not blocks;
- remaining actionable work is mostly attribute detail (165 fields) plus a few
  long-tail configuration objects.

Treat these numbers as a regression baseline, not as evidence of complete format
coverage. A diverse corpus of real customer projects is still required.

## Current lab limitations

- Attribute kinds are inferred heuristically; UUIDs are recognised only in the
  standard 36-character form.
- Node matching in `diff` uses UUIDs. Nodes without a UUID are compared by
  structural path, so reordering UUID-less wrappers can look like a change.
- The lab reports XML evidence, not Loxone semantics. Labels, units, and
  validation rules still require KB/decompilation research.
- `.zip` snapshots are accepted only when they contain `sps0.LoxCC` or another
  `.LoxCC`/`.Loxone` entry.

## Suggested implementation order

1. **Block catalog expansion** — run `scan`/`check`/`gen` on a diverse corpus,
   refine the generated schemas, and add missing child configs.
2. **Real device provisioning** — reverse-engineer Tree/Air pairing calls and
   the exact tree-node XML written by Config; replace mock serials and
   hand-built nodes.
3. **New project creation** — derive the minimal valid Document/Program skeleton
   from the smallest project in the corpus, then add structural nodes
   (Miniserver, pages, rooms, categories).
4. **Visualization authoring** — collect projects with Visu pages, diff edits
   made in the Loxone app/Config, and add model/editor support.
