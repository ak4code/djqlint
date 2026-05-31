//! DjQlint — Django Query Linter.
//!
//! Library crate exposing the analysis engine, rule set and reporters. The
//! `djqlint` binary is a thin wrapper around [`run`]; integration tests drive the
//! same code paths directly.

pub mod cli;
pub mod config;
pub mod diagnostic;
pub mod engine;
pub mod reporters;
pub mod rules;

use std::process::ExitCode;

use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

use cli::{Cli, OutputFormat};
use config::Config;
use diagnostic::FileReport;
use reporters::Summary;

/// Initialise `tracing`, sending logs to stderr so stdout stays machine-clean.
pub fn init_tracing(verbose: u8) {
    let default = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}

/// Run a full analysis according to `args` and return the process exit code.
pub fn run(args: Cli) -> Result<ExitCode> {
    if args.list_rules {
        print_rules();
        return Ok(ExitCode::SUCCESS);
    }

    let config = Config::load(args.config.as_deref()).context("loading configuration")?;
    let rule_set = rules::build_rules(&config).context("initialising rules")?;
    tracing::info!(rules = rule_set.len(), "active rules");

    let respect_gitignore = config.files.respect_gitignore && !args.no_gitignore;
    let files =
        engine::walker::collect_python_files(&args.paths, &config.files.exclude, respect_gitignore);
    tracing::info!(files = files.len(), "discovered python files");

    if files.is_empty() {
        tracing::warn!("no python files found under the given paths");
    }

    let reports = engine::analyze_files(&files, &rule_set);
    let summary = Summary::from_reports(&reports);

    let rendered = render(args.format, &reports)?;
    emit(&rendered, args.output.as_deref())?;

    if matches!(args.format, OutputFormat::Console) {
        if summary.total_findings == 0 {
            eprintln!("djqlint: no issues found in {} files ✔", files.len());
        } else {
            eprintln!(
                "djqlint: {} issue(s) across {} file(s)",
                summary.total_findings, summary.files_with_findings
            );
        }
    }

    let failed = summary.total_findings > 0 && !args.exit_zero;
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn render(format: OutputFormat, reports: &[FileReport]) -> Result<String> {
    match format {
        OutputFormat::Console => {
            let mut buf = String::new();
            reporters::console::render(reports, &mut buf)
                .map_err(|e| anyhow::anyhow!("rendering console report: {e}"))?;
            Ok(buf)
        }
        OutputFormat::Json => reporters::json::render(reports).context("rendering JSON report"),
        OutputFormat::Sarif => reporters::sarif::render(reports).context("rendering SARIF report"),
    }
}

fn emit(rendered: &str, output: Option<&std::path::Path>) -> Result<()> {
    match output {
        Some(path) => {
            std::fs::write(path, rendered)
                .with_context(|| format!("writing report to {}", path.display()))?;
            tracing::info!(path = %path.display(), "report written");
        }
        None => {
            print!("{rendered}");
        }
    }
    Ok(())
}

fn print_rules() {
    println!("DjQlint built-in rules:\n");
    for (code, name, description) in rules::all_rule_metadata() {
        println!("  {code}  {name}");
        println!("        {description}\n");
    }
}
