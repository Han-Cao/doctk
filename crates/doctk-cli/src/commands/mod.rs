//! CLI feature registry.
//!
//! Each feature module exposes a clap [`Command`] and a `run` function.
//! Adding a new CLI tool means registering it in [`build_cli`] and
//! [`dispatch`] — nothing else in the shell changes.

use clap::ArgMatches;
use doctk_core::{DoctkError, Result};

pub mod case_converter;
pub mod diff_checker;
pub mod markdown_text;
pub mod markdown_tsv;
pub mod pdf_checker;

pub fn build_cli() -> clap::Command {
    clap::Command::new("doctk")
        .about("Document processing tools: tables, diff, case conversion, and PDF preflight")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(diff_checker::cli())
        .subcommand(case_converter::cli())
        .subcommand(markdown_tsv::cli())
        .subcommand(markdown_text::cli())
        .subcommand(pdf_checker::cli())
}

pub fn dispatch(matches: &ArgMatches) -> Result<i32> {
    match matches.subcommand() {
        Some(("diff", sub)) => diff_checker::run(sub),
        Some(("case", sub)) => case_converter::run(sub),
        Some(("table", sub)) => markdown_tsv::run(sub),
        Some(("md2text", sub)) => markdown_text::run(sub),
        Some(("pdf", sub)) => pdf_checker::run(sub),
        _ => Err(DoctkError::InvalidArgument(
            "unknown command; run `doctk --help` for usage".to_string(),
        )),
    }
}

/// Read UTF-8 text from a file path, or stdin when path is `-`.
pub(crate) fn read_input(path: &str) -> Result<String> {
    use std::io::Read;
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(DoctkError::Io)?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).map_err(DoctkError::Io)
    }
}

/// Write text to stdout or a file.
pub(crate) fn write_output(path: &str, content: &str) -> Result<()> {
    use std::io::Write;
    if path == "-" {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(content.as_bytes())
            .map_err(DoctkError::Io)?;
        Ok(())
    } else {
        std::fs::write(path, content).map_err(DoctkError::Io)
    }
}
