//! DjQlint binary entry point — a thin wrapper around [`djqlint::run`].

use std::process::ExitCode;

use clap::Parser;

use djqlint::cli::Cli;

fn main() -> ExitCode {
    let args = Cli::parse();
    djqlint::init_tracing(args.verbose);

    match djqlint::run(args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("djqlint: error: {err:#}");
            ExitCode::from(2)
        }
    }
}
