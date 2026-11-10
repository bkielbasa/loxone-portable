use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use loxone_core::{serialize_raw_node, serialize_raw_nodes};
use loxone_format_lab::corpus::{load_file, FileSummary};
use loxone_format_lab::diff::{diff_files, render_text as render_diff, DiffOptions};
use loxone_format_lab::dom::{node_title, node_type, node_uuid, walk};
use loxone_format_lab::gen::{generate_rust, GenOptions};
use loxone_format_lab::report::{
    render_check, render_markdown, CheckReport, Coverage, MarkdownOptions,
};
use loxone_format_lab::roundtrip::{
    build_comprehensive_mutation_project, render_roundtrip_markdown, verify_roundtrip,
    MutationManifest,
};
use loxone_format_lab::stats::{scan, Totals, TypeStats};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "loxone-format-lab",
    version,
    about = "Reverse-engineering toolkit for Loxone .Loxone / .LoxCC project files",
    long_about = "Build a corpus of real Loxone projects and use it to discover block types, \
                  attributes, pins, and the exact XML changes produced by Loxone Config."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Aggregate type/attribute/pin statistics across a project corpus.
    Scan(ScanArgs),
    /// Print raw XML for matching nodes (by UUID, type, tag, title, or path).
    Dump(DumpArgs),
    /// Show the structural XML difference between two project saves.
    Diff(DiffArgs),
    /// Compare corpus observations against the baked-in block schemas.
    Check(CheckArgs),
    /// Generate starter Rust `schema!` snippets for unknown types.
    Gen(GenArgs),
    /// Apply comprehensive E2E mutations, verify lossless round-trip, and compare with a Loxone Config 17 re-saved file.
    Roundtrip(RoundtripArgs),
}

#[derive(Args)]
struct ScanArgs {
    /// Files or directories to scan (default: ./corpus).
    paths: Vec<PathBuf>,
    /// Emit JSON instead of Markdown.
    #[arg(long)]
    json: bool,
    /// Write the report to a file instead of stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Only include types observed at least this many times.
    #[arg(long, default_value_t = 1)]
    min_count: usize,
    /// Limit the number of type sections (0 = no limit).
    #[arg(long, default_value_t = 0)]
    max_types: usize,
    /// Number of example values per attribute/pin.
    #[arg(long, default_value_t = 4)]
    examples: usize,
}

#[derive(Args)]
struct DumpArgs {
    /// Project file (.Loxone, .LoxCC, or Miniserver .zip).
    file: PathBuf,
    /// Match a node UUID exactly.
    #[arg(long)]
    uuid: Option<String>,
    /// Match a node Type exactly (e.g. LightController2).
    #[arg(long = "type")]
    type_name: Option<String>,
    /// Match an XML tag exactly (e.g. IoData).
    #[arg(long)]
    tag: Option<String>,
    /// Match a title substring, case-insensitive.
    #[arg(long)]
    title: Option<String>,
    /// Match a DOM path substring (e.g. Program/C[Page]).
    #[arg(long)]
    path: Option<String>,
    /// Maximum number of matching nodes to print.
    #[arg(long, default_value_t = 50)]
    limit: usize,
    /// Print every match (overrides --limit).
    #[arg(long)]
    all: bool,
}

#[derive(Args)]
struct DiffArgs {
    /// Old project save.
    old: PathBuf,
    /// New project save (same project after one edit).
    new: PathBuf,
    /// Additional attributes to ignore (comma separated, repeatable).
    #[arg(long = "ignore-attr", value_delimiter = ',')]
    ignore_attributes: Vec<String>,
    /// Restrict the report to one type.
    #[arg(long)]
    only_type: Option<String>,
    /// Emit JSON instead of Markdown.
    #[arg(long)]
    json: bool,
    /// Write the report to a file instead of stdout.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct CheckArgs {
    /// Files or directories to scan (default: ./corpus).
    paths: Vec<PathBuf>,
    /// Emit JSON instead of Markdown.
    #[arg(long)]
    json: bool,
    /// Write the report to a file instead of stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Exit with status 1 when any schema gap is found.
    #[arg(long)]
    fail: bool,
}

#[derive(Args)]
struct GenArgs {
    /// Files or directories to scan (default: ./corpus).
    paths: Vec<PathBuf>,
    /// Generate only these types (repeatable). Default: all unknown types.
    #[arg(long = "type")]
    types: Vec<String>,
    /// Also generate types that already have a schema.
    #[arg(long)]
    include_known: bool,
    /// Also generate metadata/caption/section types that do not look like blocks.
    #[arg(long)]
    include_non_block: bool,
    /// Keep structural attributes such as U, Title, Px, Py, Cl, Ref.
    #[arg(long)]
    include_generic_attributes: bool,
    /// Write the snippet to a file instead of stdout.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct RoundtripArgs {
    /// Source .Loxone project to mutate and verify (default: Sample.Loxone).
    #[arg(default_value = "Sample.Loxone")]
    source: PathBuf,
    /// Directory to write mutated.Loxone and manifest.json (default: target/e2e-roundtrip).
    #[arg(long, default_value = "target/e2e-roundtrip")]
    out_dir: PathBuf,
    /// Optional path to a file re-saved by Loxone Config 17 inside Parallels for comparison.
    #[arg(long)]
    config_saved: Option<PathBuf>,
    /// Emit JSON instead of Markdown.
    #[arg(long)]
    json: bool,
    /// Write the verification report to a file instead of stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Exit with status 1 if any authored object was dropped.
    #[arg(long)]
    fail: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Scan(args) => run_scan(args),
        Command::Dump(args) => run_dump(args),
        Command::Diff(args) => run_diff(args),
        Command::Check(args) => run_check(args),
        Command::Gen(args) => run_gen(args),
        Command::Roundtrip(args) => run_roundtrip(args),
    }
}

fn run_scan(args: ScanArgs) -> Result<()> {
    let paths = paths_or_default(args.paths);
    let stats = scan(&paths)?;
    let content = if args.json {
        let coverage = Coverage::build(&stats);
        serde_json::to_string_pretty(&ScanJson {
            totals: &stats.totals,
            files: &stats.files,
            errors: &stats.errors,
            coverage,
            types: &stats.types,
        })?
    } else {
        render_markdown(
            &stats,
            &MarkdownOptions {
                min_count: args.min_count,
                max_types: args.max_types,
                examples: args.examples,
            },
        )
    };
    write_or_print(args.out.as_deref(), &content)
}

fn run_check(args: CheckArgs) -> Result<()> {
    let paths = paths_or_default(args.paths);
    let stats = scan(&paths)?;
    let coverage = Coverage::build(&stats);
    let report = CheckReport::build(&stats, coverage);
    let content = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        render_check(&report)
    };
    write_or_print(args.out.as_deref(), &content)?;
    if args.fail && report.has_issues() {
        std::process::exit(1);
    }
    Ok(())
}

fn run_gen(args: GenArgs) -> Result<()> {
    let paths = paths_or_default(args.paths);
    let stats = scan(&paths)?;
    let content = generate_rust(
        &stats,
        &GenOptions {
            types: args.types,
            include_known: args.include_known,
            include_non_block: args.include_non_block,
            include_generic_attributes: args.include_generic_attributes,
        },
    )?;
    write_or_print(args.out.as_deref(), &content)
}

fn run_diff(args: DiffArgs) -> Result<()> {
    let mut options = DiffOptions::default();
    options.ignore_attributes.extend(args.ignore_attributes);
    options.only_type = args.only_type;

    let report = diff_files(&args.old, &args.new, &options)?;
    let content = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        render_diff(&report)
    };
    write_or_print(args.out.as_deref(), &content)
}

fn run_roundtrip(args: RoundtripArgs) -> Result<()> {
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("failed to create {}", args.out_dir.display()))?;
    let mutated_path = args.out_dir.join("mutated.Loxone");
    let manifest_path = args.out_dir.join("manifest.json");

    let (mutated, manifest) = if args.config_saved.is_some() && mutated_path.exists() && manifest_path.exists() {
        let proj = loxone_core::parser::load_loxone_file(&mutated_path)
            .with_context(|| format!("failed to load {}", mutated_path.display()))?;
        let mf_raw = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?;
        let mf: MutationManifest = serde_json::from_str(&mf_raw)?;
        (proj, mf)
    } else {
        let base = loxone_core::parser::load_loxone_file(&args.source)
            .with_context(|| format!("failed to load {}", args.source.display()))?;
        let (mutated, manifest) = build_comprehensive_mutation_project(&base)?;
        let xml = loxone_core::writer::serialize_loxone_xml(&mutated)?;
        std::fs::write(&mutated_path, xml)
            .with_context(|| format!("failed to write {}", mutated_path.display()))?;
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)
            .with_context(|| format!("failed to write {}", manifest_path.display()))?;
        (mutated, manifest)
    };

    let report = verify_roundtrip(
        &mutated,
        &manifest,
        Some(&mutated_path),
        args.config_saved.as_deref(),
    )?;

    let content = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        render_roundtrip_markdown(&report)
    };
    write_or_print(args.out.as_deref(), &content)?;

    if args.fail {
        if !report.internal_roundtrip_passed {
            std::process::exit(1);
        }
        if let Some(cfg) = &report.config_roundtrip {
            if !cfg.config_accepted_all_authored {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn run_dump(args: DumpArgs) -> Result<()> {
    let loaded = load_file(&args.file)?;
    let has_filter = args.uuid.is_some()
        || args.type_name.is_some()
        || args.tag.is_some()
        || args.title.is_some()
        || args.path.is_some();

    if !has_filter {
        print!("{}", serialize_raw_nodes(&loaded.roots));
        return Ok(());
    }

    let title_query = args.title.as_ref().map(|s| s.to_ascii_lowercase());
    let path_query = args.path.as_deref();
    let mut matches: Vec<(String, &loxone_core::model::RawNode)> = Vec::new();
    walk(&loaded.roots, "", &mut |node, path| {
        if let Some(uuid) = &args.uuid {
            if node_uuid(node) != Some(uuid.as_str()) {
                return;
            }
        }
        if let Some(type_name) = &args.type_name {
            if node_type(node) != Some(type_name.as_str()) {
                return;
            }
        }
        if let Some(tag) = &args.tag {
            if &node.tag != tag {
                return;
            }
        }
        if let Some(query) = &title_query {
            let Some(title) = node_title(node) else {
                return;
            };
            if !title.to_ascii_lowercase().contains(query) {
                return;
            }
        }
        if let Some(query) = path_query {
            if !path.contains(query) {
                return;
            }
        }
        matches.push((path.to_string(), node));
    });

    if matches.is_empty() {
        bail!("no nodes matched the given filters in {}", args.file.display());
    }

    let limit = if args.all { matches.len() } else { args.limit };
    for (path, node) in matches.iter().take(limit) {
        println!("<!-- path: {path} -->");
        print!("{}", serialize_raw_node(node));
        println!();
    }
    if !args.all && matches.len() > limit {
        eprintln!(
            "… {} more match(es); use --all or increase --limit",
            matches.len() - limit
        );
    }
    Ok(())
}

#[derive(Serialize)]
struct ScanJson<'a> {
    totals: &'a Totals,
    files: &'a [FileSummary],
    errors: &'a [String],
    coverage: Coverage,
    types: &'a BTreeMap<String, TypeStats>,
}

fn paths_or_default(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    if paths.is_empty() {
        vec![PathBuf::from("corpus")]
    } else {
        paths
    }
}

fn write_or_print(out: Option<&Path>, content: &str) -> Result<()> {
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
        }
        std::fs::write(path, content)
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        print!("{content}");
    }
    Ok(())
}
