//! Command-line interface (clap derive API).

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "djqlint",
    version,
    about = "Detect inefficient Django ORM queries and database anti-patterns in Python code.",
    long_about = None,
)]
pub struct Cli {
    /// Files or directories to analyse.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Output format.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Console)]
    pub format: OutputFormat,

    /// Path to a `.djqlint.toml` config file (auto-discovered if omitted).
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// Write the report to a file instead of stdout.
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Do not honour `.gitignore`/`.ignore` files while walking.
    #[arg(long)]
    pub no_gitignore: bool,

    /// Print the built-in rules and exit.
    #[arg(long)]
    pub list_rules: bool,

    /// Always exit with status 0, even when findings are reported (useful in CI
    /// when you only want the SARIF upload, not a failing job).
    #[arg(long)]
    pub exit_zero: bool,

    /// Increase logging verbosity (-v, -vv).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable, source-annotated output (miette).
    Console,
    /// Machine-readable JSON.
    Json,
    /// SARIF 2.1.0 — for GitHub Advanced Security, SonarQube, etc.
    Sarif,
}
