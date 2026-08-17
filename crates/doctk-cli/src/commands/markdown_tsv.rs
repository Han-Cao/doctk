//! CLI feature: markdown table <-> TSV conversion.

use clap::{Arg, ArgMatches, Command};
use doctk_core::features::markdown_tsv;
use doctk_core::Result;

use super::{read_input, write_output};

pub fn cli() -> Command {
    Command::new("table")
        .about("Convert between markdown tables and TSV")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("md2tsv")
                .about("Convert a markdown table to TSV")
                .arg(
                    Arg::new("input")
                        .value_name("INPUT")
                        .default_value("-")
                        .help("Input file, or `-` for stdin"),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("OUTPUT")
                        .default_value("-")
                        .help("Output file, or `-` for stdout"),
                ),
        )
        .subcommand(
            Command::new("tsv2md")
                .about("Convert TSV to a markdown table")
                .arg(
                    Arg::new("input")
                        .value_name("INPUT")
                        .default_value("-")
                        .help("Input file, or `-` for stdin"),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("OUTPUT")
                        .default_value("-")
                        .help("Output file, or `-` for stdout"),
                ),
        )
}

pub fn run(matches: &ArgMatches) -> Result<i32> {
    match matches.subcommand() {
        Some(("md2tsv", sub)) => {
            let input = read_input(
                sub.get_one::<String>("input")
                    .map(String::as_str)
                    .unwrap_or("-"),
            )?;
            let output = markdown_tsv::markdown_to_tsv(&input)?;
            write_output(
                sub.get_one::<String>("output")
                    .map(String::as_str)
                    .unwrap_or("-"),
                &output,
            )?;
        }
        Some(("tsv2md", sub)) => {
            let input = read_input(
                sub.get_one::<String>("input")
                    .map(String::as_str)
                    .unwrap_or("-"),
            )?;
            let output = markdown_tsv::tsv_to_markdown(&input)?;
            write_output(
                sub.get_one::<String>("output")
                    .map(String::as_str)
                    .unwrap_or("-"),
                &output,
            )?;
        }
        _ => {
            return Err(doctk_core::DoctkError::InvalidArgument(
                "expected `md2tsv` or `tsv2md` subcommand".to_string(),
            ));
        }
    }
    Ok(0)
}
