//! Validates an import document and writes an immutable catalog snapshot.
//!
//! This tool is for maintainers and release automation. It never fabricates
//! records: the input must contain verified metadata, official HTTPS sources,
//! and explicit hardware compatibility claims. Invalid documents are rejected
//! before any file is created.
//!
//! Run from `src-tauri`:
//!
//! ```powershell
//! # Validate only, without writing a database
//! cargo run --example import_catalog -- --input catalog.json --validate-only
//!
//! # Write a standalone snapshot
//! cargo run --example import_catalog -- --input catalog.json --output catalog.sqlite3
//!
//! # Install a snapshot into the per-user update directory
//! cargo run --example import_catalog -- --input catalog.json --updates-dir "%APPDATA%\com.opendrvr.desktop\catalog-updates"
//! ```
//!
//! Existing output files are only replaced with `--force`. The runtime opens
//! snapshots read-only; updates are new files, never in-place edits.

use std::fs;
use std::path::PathBuf;

use opendrvr_lib::catalog::import::build_snapshot;
use opendrvr_lib::catalog::model::CatalogDocument;
use opendrvr_lib::catalog::validate::validate_document;

#[derive(Debug, Default)]
struct Arguments {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    updates_dir: Option<PathBuf>,
    validate_only: bool,
    force: bool,
    help: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = parse_arguments(std::env::args().skip(1))?;

    if arguments.help {
        print_usage();
        return Ok(());
    }

    let input = arguments
        .input
        .as_deref()
        .ok_or("missing required --input <catalog.json>")?;

    let contents = fs::read_to_string(input)?;
    let document: CatalogDocument = serde_json::from_str(&contents)?;

    let report = validate_document(&document);
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }

    if !report.is_valid() {
        for error in &report.errors {
            eprintln!("error: {error}");
        }
        return Err(format!(
            "catalog document rejected: {} error(s)",
            report.errors.len()
        )
        .into());
    }

    if arguments.validate_only {
        println!(
            "valid document (catalog v{}, {} manufacturer(s), {} hardware ID(s), {} source(s), {} driver(s), {} warning(s))",
            document.catalog_version,
            document.manufacturers.len(),
            document.hardware_devices.len(),
            document.driver_sources.len(),
            document.drivers.len(),
            report.warnings.len()
        );
        return Ok(());
    }

    let output = resolve_output(&arguments, &document)?;
    let summary =
        tauri::async_runtime::block_on(build_snapshot(&output, &document, arguments.force))?;

    println!(
        "wrote {} (schema v{}, catalog v{}, {} manufacturer(s), {} hardware ID(s), {} source(s), {} driver(s), {} warning(s))",
        output.display(),
        summary.schema_version,
        summary.catalog_version,
        summary.manufacturer_count,
        summary.hardware_device_count,
        summary.driver_source_count,
        summary.driver_count,
        summary.warning_count
    );

    Ok(())
}

fn resolve_output(
    arguments: &Arguments,
    document: &CatalogDocument,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match (&arguments.output, &arguments.updates_dir) {
        (Some(_), Some(_)) => Err("use either --output or --updates-dir, not both".into()),
        (Some(output), None) => Ok(output.clone()),
        (None, Some(directory)) => {
            fs::create_dir_all(directory)?;
            Ok(directory.join(format!("catalog-{}.sqlite3", document.catalog_version)))
        }
        (None, None) => Err("missing output: use --output <path> or --updates-dir <dir>".into()),
    }
}

fn parse_arguments<I>(arguments: I) -> Result<Arguments, Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = String>,
{
    let mut parsed = Arguments::default();
    let mut arguments = arguments.into_iter();

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--input" => parsed.input = Some(PathBuf::from(next_value(&mut arguments, "--input")?)),
            "--output" => {
                parsed.output = Some(PathBuf::from(next_value(&mut arguments, "--output")?))
            }
            "--updates-dir" => {
                parsed.updates_dir =
                    Some(PathBuf::from(next_value(&mut arguments, "--updates-dir")?))
            }
            "--validate-only" => parsed.validate_only = true,
            "--force" => parsed.force = true,
            "--help" | "-h" => parsed.help = true,
            other => return Err(format!("unknown argument '{other}'").into()),
        }
    }

    if parsed.validate_only && (parsed.output.is_some() || parsed.updates_dir.is_some()) {
        return Err("--validate-only cannot be combined with an output option".into());
    }

    Ok(parsed)
}

fn next_value<I>(arguments: &mut I, name: &str) -> Result<String, Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    arguments
        .next()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| format!("{name} requires a value").into())
}

fn print_usage() {
    println!(
        "Usage: import_catalog --input <catalog.json> [options]\n\
         \n\
         Options:\n\
           --output <path>       Write the snapshot to this path\n\
           --updates-dir <dir>   Write catalog-<version>.sqlite3 into a per-user update directory\n\
           --validate-only       Validate the document without writing a database\n\
           --force               Replace an existing output file\n\
           --help, -h            Show this help"
    );
}
